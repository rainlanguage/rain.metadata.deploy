//! Classification of deployed subgraphs into *retained* and *reaping candidate*.
//!
//! Pure: takes an already-fetched listing, a policy and a clock reading, and
//! returns a verdict per deployed version. No IO, and no ability to delete.
//!
//! The signal is **supersession**, not usage. Goldsky's admin API exposes no
//! per-subgraph query count, bandwidth or last-query timestamp (see the crate
//! docs), so "nobody is calling this" cannot be established from the API. What
//! can be established is that a version has been replaced by a newer one under
//! the same name and nothing points at it any more — which is exactly the
//! residue the idempotent-by-name deploy leaves behind.

use std::collections::{BTreeMap, BTreeSet};

/// Milliseconds per day. Goldsky reports `created_at` in epoch millis.
const MS_PER_DAY: i64 = 86_400_000;

/// What a listing row actually is.
///
/// Goldsky's listing returns two kinds of row under one shape: real deployed
/// versions, and tag aliases whose `version` is the tag label and which point
/// at some target version. Only deployments are ever classified; an alias is
/// reported but never a candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// A real deployed version.
    Deployment,
    /// An alias row: `name/<version>` resolves to `name/<target_version>`.
    /// `target_version` is `None` for a tag that points at nothing.
    TagAlias { target_version: Option<String> },
}

/// One row of a Goldsky subgraph listing, reduced to what classification needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubgraphEntry {
    pub name: String,
    pub version: String,
    pub network: String,
    pub kind: Kind,
    pub paused: bool,
    /// Creation time of the newest deployment backing this row, epoch millis.
    /// `None` when Goldsky returned no deployment records, in which case the
    /// age is unknown and no age-based judgement is possible.
    pub created_at_ms: Option<i64>,
    pub graphql_endpoint: String,
}

impl SubgraphEntry {
    /// `name/version`, the identifier Goldsky itself uses for a deployed
    /// version and the string a human would hand to the Goldsky CLI.
    pub fn name_and_version(&self) -> String {
        format!("{}/{}", self.name, self.version)
    }

    pub fn is_tag_alias(&self) -> bool {
        matches!(self.kind, Kind::TagAlias { .. })
    }
}

/// Why a deployed version is being kept. Every retention records its reason so
/// a human reading the report can audit the decision rather than trust it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Retained {
    /// The caller named this version explicitly as protected.
    Pinned,
    /// A tag on this name resolves to this version, so consumers following the
    /// tag reach it.
    TagTarget,
    /// Goldsky returned no deployment record, so the age is unknown and this
    /// version cannot be judged.
    UnknownAge,
    /// The newest version for its name. The live deploy is never a candidate.
    Newest,
    /// Superseded, but created too recently to be inside the reaping window.
    WithinAgeWindow { age_days: i64 },
}

/// The verdict for one deployed version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Retained(Retained),
    /// Superseded by a newer version of the same name, untagged, unpinned and
    /// older than the configured window. A candidate for a human to reap; this
    /// crate never acts on it.
    Candidate {
        age_days: i64,
    },
}

impl Verdict {
    pub fn is_candidate(&self) -> bool {
        matches!(self, Verdict::Candidate { .. })
    }
}

/// The verdict for one deployed version, with the row it was reached from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    pub entry: SubgraphEntry,
    pub verdict: Verdict,
}

/// An alias row, reported so a human can see which URL is canonical for a name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagRow {
    pub name: String,
    pub label: String,
    /// `None` for a tag pointing at nothing — a dangling alias worth surfacing.
    pub target_version: Option<String>,
}

impl TagRow {
    pub fn is_dangling(&self) -> bool {
        self.target_version.is_none()
    }
}

/// The reaping policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    /// A superseded version must be at least this many days old before it is
    /// reported as a candidate.
    pub min_age_days: i64,
    /// `name/version` strings that are never candidates whatever else holds.
    pub pinned: BTreeSet<String>,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            min_age_days: 30,
            pinned: BTreeSet::new(),
        }
    }
}

/// The classified listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// One assessment per deployed version, in the order given.
    pub deployments: Vec<Assessment>,
    /// One row per tag alias, in the order given.
    pub tags: Vec<TagRow>,
}

impl Report {
    pub fn candidates(&self) -> impl Iterator<Item = &Assessment> {
        self.deployments.iter().filter(|a| a.verdict.is_candidate())
    }

    pub fn dangling_tags(&self) -> impl Iterator<Item = &TagRow> {
        self.tags.iter().filter(|t| t.is_dangling())
    }
}

/// Keep only the rows whose name starts with one of `prefixes`.
///
/// Prefix rather than substring: the deployed names are `<subgraph>-<network>`,
/// so a prefix anchors on the subgraph and cannot be satisfied by a name that
/// merely contains it (`some-other-metaboard-thing`).
///
/// Repeatable because the deployed estate is not uniform. The current deploy
/// rule produces `metaboard-<network>`, but older live deploys use other stems
/// (`metadata-<network>`, `mb-<network>-<address>`), so enumerating the whole
/// estate takes more than one prefix. An empty prefix list matches everything.
pub fn filter_by_name_prefixes(
    entries: &[SubgraphEntry],
    prefixes: &[String],
) -> Vec<SubgraphEntry> {
    entries
        .iter()
        .filter(|e| prefixes.is_empty() || prefixes.iter().any(|p| e.name.starts_with(p)))
        .cloned()
        .collect()
}

/// Whole days elapsed between `created_at_ms` and `now_ms`, floored, never
/// negative: a timestamp in the future reads as age zero rather than as a
/// negative age that would slip past the window check.
pub fn age_days(created_at_ms: i64, now_ms: i64) -> i64 {
    let elapsed = now_ms.saturating_sub(created_at_ms);
    if elapsed < 0 { 0 } else { elapsed / MS_PER_DAY }
}

/// Classify every row.
///
/// Retention wins every tie. A deployed version is a candidate only when it is
/// *all* of: not a tag alias, not pinned, not a tag target, of known age, not
/// the newest for its name, and at least `min_age_days` old. Anything that
/// cannot be positively established is retained, because a false retention
/// costs nothing and a false candidate risks a live subgraph.
pub fn classify(entries: &[SubgraphEntry], policy: &Policy, now_ms: i64) -> Report {
    // Versions any alias resolves to, keyed by the name the alias sits under.
    let tag_targets: BTreeSet<(&str, &str)> = entries
        .iter()
        .filter_map(|e| match &e.kind {
            Kind::TagAlias {
                target_version: Some(target),
            } => Some((e.name.as_str(), target.as_str())),
            _ => None,
        })
        .collect();

    // Newest creation time per name, over deployment rows only. A name with no
    // dated deployment gets no entry, so nothing under it is judged "not
    // newest".
    let mut newest: BTreeMap<&str, i64> = BTreeMap::new();
    for e in entries.iter().filter(|e| !e.is_tag_alias()) {
        if let Some(created) = e.created_at_ms {
            newest
                .entry(e.name.as_str())
                .and_modify(|c| {
                    if created > *c {
                        *c = created;
                    }
                })
                .or_insert(created);
        }
    }

    let mut deployments = Vec::new();
    let mut tags = Vec::new();

    for e in entries {
        match &e.kind {
            Kind::TagAlias { target_version } => tags.push(TagRow {
                name: e.name.clone(),
                label: e.version.clone(),
                target_version: target_version.clone(),
            }),
            Kind::Deployment => {
                let verdict = if policy.pinned.contains(&e.name_and_version()) {
                    Verdict::Retained(Retained::Pinned)
                } else if tag_targets.contains(&(e.name.as_str(), e.version.as_str())) {
                    Verdict::Retained(Retained::TagTarget)
                } else {
                    match e.created_at_ms {
                        None => Verdict::Retained(Retained::UnknownAge),
                        Some(created) => {
                            // Ties count as newest: two versions sharing the
                            // newest timestamp are both retained.
                            if newest.get(e.name.as_str()) == Some(&created) {
                                Verdict::Retained(Retained::Newest)
                            } else {
                                let age = age_days(created, now_ms);
                                if age < policy.min_age_days {
                                    Verdict::Retained(Retained::WithinAgeWindow { age_days: age })
                                } else {
                                    Verdict::Candidate { age_days: age }
                                }
                            }
                        }
                    }
                };
                deployments.push(Assessment {
                    entry: e.clone(),
                    verdict,
                });
            }
        }
    }

    Report { deployments, tags }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = MS_PER_DAY;
    /// An arbitrary fixed "now" so ages in tests are exact, not clock-dependent.
    const NOW: i64 = 1_700_000_000_000;

    fn deployment(name: &str, version: &str, created_at_ms: Option<i64>) -> SubgraphEntry {
        SubgraphEntry {
            name: name.to_string(),
            version: version.to_string(),
            network: "base".to_string(),
            kind: Kind::Deployment,
            paused: false,
            created_at_ms,
            graphql_endpoint: format!("/api/public/p/subgraphs/{name}/{version}/gn"),
        }
    }

    fn alias(name: &str, label: &str, target: Option<&str>) -> SubgraphEntry {
        SubgraphEntry {
            name: name.to_string(),
            version: label.to_string(),
            network: "base".to_string(),
            kind: Kind::TagAlias {
                target_version: target.map(String::from),
            },
            paused: false,
            created_at_ms: None,
            graphql_endpoint: String::new(),
        }
    }

    fn policy(min_age_days: i64, pinned: &[&str]) -> Policy {
        Policy {
            min_age_days,
            pinned: pinned.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn verdict_for<'a>(report: &'a Report, name_and_version: &str) -> &'a Verdict {
        &report
            .deployments
            .iter()
            .find(|a| a.entry.name_and_version() == name_and_version)
            .unwrap_or_else(|| panic!("no assessment for {name_and_version}"))
            .verdict
    }

    // ---------- age_days ----------

    #[test]
    fn age_days_floors_to_whole_days() {
        assert_eq!(age_days(NOW - DAY - 1, NOW), 1);
        assert_eq!(age_days(NOW - (2 * DAY) + 1, NOW), 1);
        assert_eq!(age_days(NOW - 2 * DAY, NOW), 2);
    }

    #[test]
    fn age_days_of_now_is_zero() {
        assert_eq!(age_days(NOW, NOW), 0);
    }

    #[test]
    fn age_days_never_goes_negative_for_a_future_timestamp() {
        // A clock skew must not produce a negative age that slips past the
        // `age < min_age_days` window check as if it were very old.
        assert_eq!(age_days(NOW + 5 * DAY, NOW), 0);
        assert_eq!(age_days(i64::MAX, NOW), 0);
    }

    // ---------- the single-version case ----------

    #[test]
    fn a_lone_version_is_newest_and_never_a_candidate() {
        let entries = vec![deployment("metaboard-base", "v1", Some(NOW - 900 * DAY))];
        let report = classify(&entries, &policy(30, &[]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/v1"),
            &Verdict::Retained(Retained::Newest)
        );
        assert_eq!(report.candidates().count(), 0);
    }

    // ---------- supersession ----------

    #[test]
    fn an_older_version_superseded_by_a_newer_one_is_a_candidate() {
        let entries = vec![
            deployment("metaboard-base", "old", Some(NOW - 100 * DAY)),
            deployment("metaboard-base", "new", Some(NOW - DAY)),
        ];
        let report = classify(&entries, &policy(30, &[]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/old"),
            &Verdict::Candidate { age_days: 100 }
        );
        assert_eq!(
            verdict_for(&report, "metaboard-base/new"),
            &Verdict::Retained(Retained::Newest)
        );
    }

    #[test]
    fn supersession_is_scoped_to_one_name() {
        // A newer version of a DIFFERENT name must not supersede this one.
        let entries = vec![
            deployment("metaboard-base", "only", Some(NOW - 100 * DAY)),
            deployment("metaboard-flare", "newer", Some(NOW - DAY)),
        ];
        let report = classify(&entries, &policy(30, &[]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/only"),
            &Verdict::Retained(Retained::Newest)
        );
        assert_eq!(report.candidates().count(), 0);
    }

    #[test]
    fn only_the_newest_survives_among_many() {
        let entries = vec![
            deployment("metaboard-base", "a", Some(NOW - 300 * DAY)),
            deployment("metaboard-base", "b", Some(NOW - 200 * DAY)),
            deployment("metaboard-base", "c", Some(NOW - 100 * DAY)),
        ];
        let report = classify(&entries, &policy(30, &[]), NOW);
        let candidates: Vec<String> = report
            .candidates()
            .map(|a| a.entry.name_and_version())
            .collect();
        assert_eq!(candidates, vec!["metaboard-base/a", "metaboard-base/b"]);
        assert_eq!(
            verdict_for(&report, "metaboard-base/c"),
            &Verdict::Retained(Retained::Newest)
        );
    }

    #[test]
    fn newest_is_by_timestamp_not_by_listing_order() {
        // The newest row is deliberately listed FIRST, so an implementation
        // that took "last seen" or "first seen" rather than max would differ.
        let entries = vec![
            deployment("metaboard-base", "new", Some(NOW - DAY)),
            deployment("metaboard-base", "old", Some(NOW - 100 * DAY)),
        ];
        let report = classify(&entries, &policy(30, &[]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/new"),
            &Verdict::Retained(Retained::Newest)
        );
        assert_eq!(
            verdict_for(&report, "metaboard-base/old"),
            &Verdict::Candidate { age_days: 100 }
        );
    }

    #[test]
    fn versions_tied_on_the_newest_timestamp_are_both_retained() {
        let entries = vec![
            deployment("metaboard-base", "a", Some(NOW - 100 * DAY)),
            deployment("metaboard-base", "b", Some(NOW - 100 * DAY)),
        ];
        let report = classify(&entries, &policy(30, &[]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/a"),
            &Verdict::Retained(Retained::Newest)
        );
        assert_eq!(
            verdict_for(&report, "metaboard-base/b"),
            &Verdict::Retained(Retained::Newest)
        );
        assert_eq!(report.candidates().count(), 0);
    }

    // ---------- the age window ----------

    #[test]
    fn a_superseded_version_inside_the_window_is_retained() {
        let entries = vec![
            deployment("metaboard-base", "old", Some(NOW - 29 * DAY)),
            deployment("metaboard-base", "new", Some(NOW)),
        ];
        let report = classify(&entries, &policy(30, &[]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/old"),
            &Verdict::Retained(Retained::WithinAgeWindow { age_days: 29 })
        );
    }

    #[test]
    fn the_age_window_boundary_is_inclusive_of_the_threshold() {
        // Exactly min_age_days old IS a candidate; one day younger is not.
        let at_threshold = vec![
            deployment("metaboard-base", "old", Some(NOW - 30 * DAY)),
            deployment("metaboard-base", "new", Some(NOW)),
        ];
        assert_eq!(
            verdict_for(
                &classify(&at_threshold, &policy(30, &[]), NOW),
                "metaboard-base/old"
            ),
            &Verdict::Candidate { age_days: 30 }
        );

        let below_threshold = vec![
            deployment("metaboard-base", "old", Some(NOW - 30 * DAY + 1)),
            deployment("metaboard-base", "new", Some(NOW)),
        ];
        assert_eq!(
            verdict_for(
                &classify(&below_threshold, &policy(30, &[]), NOW),
                "metaboard-base/old"
            ),
            &Verdict::Retained(Retained::WithinAgeWindow { age_days: 29 })
        );
    }

    #[test]
    fn a_zero_day_window_still_retains_the_newest() {
        let entries = vec![
            deployment("metaboard-base", "old", Some(NOW)),
            deployment("metaboard-base", "new", Some(NOW + DAY)),
        ];
        let report = classify(&entries, &policy(0, &[]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/new"),
            &Verdict::Retained(Retained::Newest)
        );
        assert_eq!(
            verdict_for(&report, "metaboard-base/old"),
            &Verdict::Candidate { age_days: 0 }
        );
    }

    // ---------- tags ----------

    #[test]
    fn a_tag_target_is_retained_even_when_superseded_and_ancient() {
        let entries = vec![
            deployment("metaboard-base", "tagged", Some(NOW - 900 * DAY)),
            deployment("metaboard-base", "new", Some(NOW)),
            alias("metaboard-base", "latest", Some("tagged")),
        ];
        let report = classify(&entries, &policy(30, &[]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/tagged"),
            &Verdict::Retained(Retained::TagTarget)
        );
        assert_eq!(report.candidates().count(), 0);
    }

    #[test]
    fn a_tag_only_protects_its_own_name() {
        // An alias under metaboard-flare naming version "shared" must not
        // protect metaboard-base/shared.
        let entries = vec![
            deployment("metaboard-base", "shared", Some(NOW - 100 * DAY)),
            deployment("metaboard-base", "new", Some(NOW)),
            alias("metaboard-flare", "latest", Some("shared")),
        ];
        let report = classify(&entries, &policy(30, &[]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/shared"),
            &Verdict::Candidate { age_days: 100 }
        );
    }

    #[test]
    fn alias_rows_are_reported_separately_and_never_assessed() {
        let entries = vec![
            deployment("metaboard-base", "v1", Some(NOW)),
            alias("metaboard-base", "latest", Some("v1")),
        ];
        let report = classify(&entries, &policy(30, &[]), NOW);
        assert_eq!(report.deployments.len(), 1);
        assert_eq!(
            report.tags,
            vec![TagRow {
                name: "metaboard-base".to_string(),
                label: "latest".to_string(),
                target_version: Some("v1".to_string()),
            }]
        );
    }

    #[test]
    fn an_alias_is_never_a_candidate_even_if_it_looks_superseded() {
        // The safety property: a row carrying a tag can never be reaped by
        // this tool, whatever its age or position.
        let entries = vec![
            alias("metaboard-base", "ancient-label", Some("v1")),
            deployment("metaboard-base", "v1", Some(NOW)),
            deployment("metaboard-base", "v2", Some(NOW + DAY)),
        ];
        let report = classify(&entries, &policy(0, &[]), NOW);
        for a in &report.deployments {
            assert!(!a.entry.is_tag_alias());
        }
        assert!(
            report
                .candidates()
                .all(|a| a.entry.name_and_version() != "metaboard-base/ancient-label")
        );
    }

    #[test]
    fn a_dangling_tag_is_reported_and_protects_nothing() {
        let entries = vec![
            deployment("metaboard-base", "old", Some(NOW - 100 * DAY)),
            deployment("metaboard-base", "new", Some(NOW)),
            alias("metaboard-base", "latest", None),
        ];
        let report = classify(&entries, &policy(30, &[]), NOW);
        assert_eq!(report.dangling_tags().count(), 1);
        // The dangling tag names no target, so nothing gains protection.
        assert_eq!(
            verdict_for(&report, "metaboard-base/old"),
            &Verdict::Candidate { age_days: 100 }
        );
    }

    #[test]
    fn an_alias_does_not_count_as_the_newest_deployment() {
        // If aliases were folded into the newest calculation, a recent alias
        // row would make a real newest deployment look superseded.
        let entries = vec![
            deployment("metaboard-base", "real", Some(NOW - 100 * DAY)),
            alias("metaboard-base", "latest", Some("real")),
        ];
        let report = classify(&entries, &policy(30, &[]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/real"),
            &Verdict::Retained(Retained::TagTarget)
        );
    }

    #[test]
    fn an_alias_carrying_its_own_timestamp_does_not_displace_the_newest() {
        // Goldsky can return deployment records on a tag row. Folding those
        // into the newest calculation would make a live, newest deployment
        // look superseded and hand it to a human as a reaping candidate.
        let mut recent_alias = alias("metaboard-base", "latest", None);
        recent_alias.created_at_ms = Some(NOW);
        let entries = vec![
            deployment("metaboard-base", "real", Some(NOW - 100 * DAY)),
            recent_alias,
        ];
        let report = classify(&entries, &policy(30, &[]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/real"),
            &Verdict::Retained(Retained::Newest)
        );
        assert_eq!(report.candidates().count(), 0);
    }

    // ---------- pinning ----------

    #[test]
    fn a_pinned_version_is_retained_however_old_and_superseded() {
        let entries = vec![
            deployment("metaboard-base", "old", Some(NOW - 900 * DAY)),
            deployment("metaboard-base", "new", Some(NOW)),
        ];
        let report = classify(&entries, &policy(30, &["metaboard-base/old"]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/old"),
            &Verdict::Retained(Retained::Pinned)
        );
        assert_eq!(report.candidates().count(), 0);
    }

    #[test]
    fn pinning_matches_the_full_name_and_version_not_the_version_alone() {
        let entries = vec![
            deployment("metaboard-base", "old", Some(NOW - 900 * DAY)),
            deployment("metaboard-base", "new", Some(NOW)),
        ];
        // Pinning a bare version, or another name's version, must not protect.
        let report = classify(&entries, &policy(30, &["old", "metaboard-flare/old"]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/old"),
            &Verdict::Candidate { age_days: 900 }
        );
    }

    // ---------- unknown age ----------

    #[test]
    fn a_version_with_no_deployment_record_is_retained_as_unknown_age() {
        let entries = vec![
            deployment("metaboard-base", "undated", None),
            deployment("metaboard-base", "new", Some(NOW)),
        ];
        let report = classify(&entries, &policy(30, &[]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/undated"),
            &Verdict::Retained(Retained::UnknownAge)
        );
        assert_eq!(report.candidates().count(), 0);
    }

    #[test]
    fn undated_rows_do_not_establish_a_newest_for_their_name() {
        let entries = vec![deployment("metaboard-base", "undated", None)];
        let report = classify(&entries, &policy(30, &[]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/undated"),
            &Verdict::Retained(Retained::UnknownAge)
        );
    }

    // ---------- precedence ----------

    #[test]
    fn pinning_takes_precedence_over_every_other_reason() {
        let entries = vec![
            deployment("metaboard-base", "old", Some(NOW - 900 * DAY)),
            deployment("metaboard-base", "new", Some(NOW)),
            alias("metaboard-base", "latest", Some("old")),
        ];
        let report = classify(&entries, &policy(30, &["metaboard-base/old"]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/old"),
            &Verdict::Retained(Retained::Pinned)
        );
    }

    #[test]
    fn a_tag_target_outranks_unknown_age() {
        let entries = vec![
            deployment("metaboard-base", "undated", None),
            alias("metaboard-base", "latest", Some("undated")),
        ];
        let report = classify(&entries, &policy(30, &[]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/undated"),
            &Verdict::Retained(Retained::TagTarget)
        );
    }

    // ---------- paused ----------

    #[test]
    fn paused_is_carried_through_but_does_not_by_itself_make_a_candidate() {
        let mut paused = deployment("metaboard-base", "only", Some(NOW - 900 * DAY));
        paused.paused = true;
        let report = classify(&[paused], &policy(30, &[]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/only"),
            &Verdict::Retained(Retained::Newest)
        );
        assert!(report.deployments[0].entry.paused);
    }

    // ---------- filtering ----------

    #[test]
    fn the_prefix_filter_anchors_at_the_start_of_the_name() {
        let entries = vec![
            deployment("metaboard-base", "v", Some(NOW)),
            deployment("not-metaboard-base", "v", Some(NOW)),
        ];
        let kept = filter_by_name_prefixes(&entries, &[String::from("metaboard")]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].name, "metaboard-base");
    }

    #[test]
    fn the_prefix_filter_accepts_several_prefixes() {
        let entries = vec![
            deployment("metaboard-base", "v", Some(NOW)),
            deployment("metadata-base", "v", Some(NOW)),
            deployment("unrelated", "v", Some(NOW)),
        ];
        let kept = filter_by_name_prefixes(
            &entries,
            &[String::from("metaboard"), String::from("metadata")],
        );
        let names: Vec<&str> = kept.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["metaboard-base", "metadata-base"]);
    }

    #[test]
    fn an_empty_prefix_list_matches_everything() {
        let entries = vec![
            deployment("metaboard-base", "v", Some(NOW)),
            deployment("unrelated", "v", Some(NOW)),
        ];
        assert_eq!(filter_by_name_prefixes(&entries, &[]).len(), 2);
    }

    #[test]
    fn the_filter_keeps_alias_rows_so_tags_still_protect_after_filtering() {
        // Dropping aliases at the filter step would silently strip the
        // protection a tag confers.
        let entries = vec![
            deployment("metaboard-base", "old", Some(NOW - 900 * DAY)),
            deployment("metaboard-base", "new", Some(NOW)),
            alias("metaboard-base", "latest", Some("old")),
        ];
        let kept = filter_by_name_prefixes(&entries, &[String::from("metaboard")]);
        let report = classify(&kept, &policy(30, &[]), NOW);
        assert_eq!(
            verdict_for(&report, "metaboard-base/old"),
            &Verdict::Retained(Retained::TagTarget)
        );
    }

    // ---------- shape ----------

    #[test]
    fn an_empty_listing_produces_an_empty_report() {
        let report = classify(&[], &policy(30, &[]), NOW);
        assert!(report.deployments.is_empty());
        assert!(report.tags.is_empty());
        assert_eq!(report.candidates().count(), 0);
    }

    #[test]
    fn name_and_version_joins_with_a_slash() {
        assert_eq!(
            deployment("metaboard-base", "0xabc-1106a15", None).name_and_version(),
            "metaboard-base/0xabc-1106a15"
        );
    }

    #[test]
    fn assessments_preserve_the_input_order_of_deployment_rows() {
        let entries = vec![
            deployment("metaboard-base", "a", Some(NOW - 300 * DAY)),
            alias("metaboard-base", "latest", Some("c")),
            deployment("metaboard-base", "b", Some(NOW - 200 * DAY)),
            deployment("metaboard-base", "c", Some(NOW - 100 * DAY)),
        ];
        let report = classify(&entries, &policy(30, &[]), NOW);
        let order: Vec<String> = report
            .deployments
            .iter()
            .map(|a| a.entry.version.clone())
            .collect();
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn the_default_policy_pins_nothing_and_uses_a_thirty_day_window() {
        let default = Policy::default();
        assert_eq!(default.min_age_days, 30);
        assert!(default.pinned.is_empty());
    }
}
