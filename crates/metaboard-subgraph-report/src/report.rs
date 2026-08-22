//! Rendering of a classified listing.
//!
//! Two forms: a human table and a machine-readable JSON document. Both state
//! explicitly that per-subgraph usage metrics are unavailable, so neither can
//! be mistaken for a usage-based report.
//!
//! Note what is deliberately absent: nothing here emits a runnable
//! `goldsky subgraph delete` command line or a shell script. The candidate
//! list is `name/version` identifiers only. Reaping is a human's decision and
//! a human's keystroke.

use crate::classify::{Assessment, Policy, Report, Retained, Verdict, age_days};
use serde_json::{Value, json};

/// Stated in every report: Goldsky's subgraph admin API exposes no query
/// count, no bandwidth and no last-query timestamp, so "unused" cannot be
/// measured and "superseded" is what is actually reported.
pub const USAGE_CAVEAT: &str = "Goldsky's subgraph admin API exposes no per-subgraph usage metrics \
(no query count, no bandwidth, no last-query timestamp). Candidates below are SUPERSEDED, not \
proven unused: each is an untagged, unpinned version that a newer version of the same name has \
replaced. Confirm nothing queries a candidate before reaping it.";

/// Stable machine-readable reason for a verdict.
pub fn reason_code(verdict: &Verdict) -> &'static str {
    match verdict {
        Verdict::Retained(Retained::Pinned) => "pinned",
        Verdict::Retained(Retained::TagTarget) => "tag-target",
        Verdict::Retained(Retained::UnknownAge) => "unknown-age",
        Verdict::Retained(Retained::Newest) => "newest",
        Verdict::Retained(Retained::WithinAgeWindow { .. }) => "within-age-window",
        Verdict::Candidate { .. } => "superseded",
    }
}

/// Human-readable one-liner for a verdict.
pub fn reason_text(verdict: &Verdict) -> String {
    match verdict {
        Verdict::Retained(Retained::Pinned) => "retained: pinned by caller".to_string(),
        Verdict::Retained(Retained::TagTarget) => "retained: a tag resolves here".to_string(),
        Verdict::Retained(Retained::UnknownAge) => {
            "retained: no deployment record, age unknown".to_string()
        }
        Verdict::Retained(Retained::Newest) => "retained: newest for this name".to_string(),
        Verdict::Retained(Retained::WithinAgeWindow { age_days }) => {
            format!("retained: superseded but only {age_days}d old")
        }
        Verdict::Candidate { age_days } => format!("CANDIDATE: superseded, {age_days}d old"),
    }
}

fn assessment_age(assessment: &Assessment, now_ms: i64) -> Option<i64> {
    assessment.entry.created_at_ms.map(|c| age_days(c, now_ms))
}

/// The machine-readable report.
pub fn render_json(
    report: &Report,
    policy: &Policy,
    name_prefixes: &[String],
    now_ms: i64,
) -> Value {
    let deployments: Vec<Value> = report
        .deployments
        .iter()
        .map(|a| {
            json!({
                "name": a.entry.name,
                "version": a.entry.version,
                "name_and_version": a.entry.name_and_version(),
                "network": a.entry.network,
                "paused": a.entry.paused,
                "created_at_ms": a.entry.created_at_ms,
                "age_days": assessment_age(a, now_ms),
                "graphql_endpoint": a.entry.graphql_endpoint,
                "verdict": if a.verdict.is_candidate() { "candidate" } else { "retained" },
                "reason": reason_code(&a.verdict),
            })
        })
        .collect();

    let tags: Vec<Value> = report
        .tags
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "label": t.label,
                "target_version": t.target_version,
                "dangling": t.is_dangling(),
            })
        })
        .collect();

    let candidates: Vec<String> = report
        .candidates()
        .map(|a| a.entry.name_and_version())
        .collect();

    json!({
        "generated_at_ms": now_ms,
        "usage_metrics_available": false,
        "caveat": USAGE_CAVEAT,
        "policy": {
            "name_prefixes": name_prefixes,
            "min_age_days": policy.min_age_days,
            "pinned": policy.pinned.iter().collect::<Vec<_>>(),
        },
        "deployments": deployments,
        "tags": tags,
        "candidates": candidates,
    })
}

/// Candidate identifiers, one `name/version` per line, for a human to act on.
pub fn render_candidates(report: &Report) -> String {
    report
        .candidates()
        .map(|a| a.entry.name_and_version())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The human table.
pub fn render_table(
    report: &Report,
    policy: &Policy,
    name_prefixes: &[String],
    now_ms: i64,
) -> String {
    let mut out = String::new();

    let prefix_display = if name_prefixes.is_empty() {
        "<all>".to_string()
    } else {
        name_prefixes.join(", ")
    };
    out.push_str(&format!(
        "Deployed subgraphs matching prefix(es) {} (window: {}d)\n\n",
        prefix_display, policy.min_age_days
    ));

    if report.deployments.is_empty() {
        out.push_str("No deployed versions matched.\n");
    } else {
        let id_width = report
            .deployments
            .iter()
            .map(|a| a.entry.name_and_version().len())
            .max()
            .unwrap_or(0);
        let net_width = report
            .deployments
            .iter()
            .map(|a| a.entry.network.len())
            .max()
            .unwrap_or(0);

        for a in &report.deployments {
            let age = match assessment_age(a, now_ms) {
                Some(d) => format!("{d}d"),
                None => "?".to_string(),
            };
            let paused = if a.entry.paused { " [paused]" } else { "" };
            out.push_str(&format!(
                "{:id_width$}  {:net_width$}  {:>5}  {}{}\n",
                a.entry.name_and_version(),
                a.entry.network,
                age,
                reason_text(&a.verdict),
                paused,
            ));
        }
    }

    if !report.tags.is_empty() {
        out.push_str("\nTag aliases\n");
        for t in &report.tags {
            match &t.target_version {
                Some(target) => out.push_str(&format!(
                    "  {}/{} -> {}/{}\n",
                    t.name, t.label, t.name, target
                )),
                None => out.push_str(&format!(
                    "  {}/{} -> [no target] (dangling)\n",
                    t.name, t.label
                )),
            }
        }
    }

    let candidate_count = report.candidates().count();
    out.push_str(&format!(
        "\n{} deployed version(s), {} tag alias(es), {} reaping candidate(s).\n",
        report.deployments.len(),
        report.tags.len(),
        candidate_count
    ));
    out.push_str(&format!("\n{USAGE_CAVEAT}\n"));

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classify::{Kind, SubgraphEntry, classify};

    const DAY: i64 = 86_400_000;
    const NOW: i64 = 1_700_000_000_000;

    fn deployment(name: &str, version: &str, created_at_ms: Option<i64>) -> SubgraphEntry {
        SubgraphEntry {
            name: name.to_string(),
            version: version.to_string(),
            network: "base".to_string(),
            kind: Kind::Deployment,
            paused: false,
            created_at_ms,
            graphql_endpoint: "/gn".to_string(),
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

    fn prefixes() -> Vec<String> {
        vec![String::from("metaboard")]
    }

    /// One superseded candidate, one newest that a `latest` alias also resolves
    /// to (so it reads as tag-target, which outranks newest), and that alias.
    fn mixed_report() -> Report {
        let entries = vec![
            deployment("metaboard-base", "old", Some(NOW - 100 * DAY)),
            deployment("metaboard-base", "new", Some(NOW)),
            alias("metaboard-base", "latest", Some("new")),
        ];
        classify(&entries, &Policy::default(), NOW)
    }

    // ---------- reason codes ----------

    #[test]
    fn every_verdict_has_a_distinct_reason_code() {
        let codes = [
            reason_code(&Verdict::Retained(Retained::Pinned)),
            reason_code(&Verdict::Retained(Retained::TagTarget)),
            reason_code(&Verdict::Retained(Retained::UnknownAge)),
            reason_code(&Verdict::Retained(Retained::Newest)),
            reason_code(&Verdict::Retained(Retained::WithinAgeWindow {
                age_days: 1,
            })),
            reason_code(&Verdict::Candidate { age_days: 1 }),
        ];
        let mut unique = codes.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), codes.len(), "reason codes must be distinct");
    }

    #[test]
    fn the_candidate_reason_code_names_supersession_not_disuse() {
        // The code is part of the contract with anything consuming the JSON;
        // it must not claim the tool measured usage.
        assert_eq!(
            reason_code(&Verdict::Candidate { age_days: 90 }),
            "superseded"
        );
    }

    #[test]
    fn reason_text_reports_the_age_it_was_given() {
        assert!(reason_text(&Verdict::Candidate { age_days: 97 }).contains("97d"));
        assert!(
            reason_text(&Verdict::Retained(Retained::WithinAgeWindow {
                age_days: 3
            }))
            .contains("3d")
        );
    }

    #[test]
    fn only_a_candidate_reads_as_a_candidate_in_its_text() {
        assert!(reason_text(&Verdict::Candidate { age_days: 1 }).contains("CANDIDATE"));
        for retained in [
            Retained::Pinned,
            Retained::TagTarget,
            Retained::UnknownAge,
            Retained::Newest,
            Retained::WithinAgeWindow { age_days: 1 },
        ] {
            let text = reason_text(&Verdict::Retained(retained));
            assert!(text.starts_with("retained:"), "unexpected text: {text}");
            assert!(!text.contains("CANDIDATE"));
        }
    }

    // ---------- json ----------

    #[test]
    fn the_json_report_declares_that_usage_metrics_are_unavailable() {
        let json = render_json(&mixed_report(), &Policy::default(), &prefixes(), NOW);
        assert_eq!(json["usage_metrics_available"], serde_json::json!(false));
        assert!(
            json["caveat"]
                .as_str()
                .unwrap()
                .contains("no per-subgraph usage")
        );
    }

    #[test]
    fn the_json_report_lists_only_candidates_under_candidates() {
        let json = render_json(&mixed_report(), &Policy::default(), &prefixes(), NOW);
        assert_eq!(
            json["candidates"],
            serde_json::json!(["metaboard-base/old"])
        );
    }

    #[test]
    fn the_json_report_carries_a_verdict_and_reason_per_deployment() {
        let json = render_json(&mixed_report(), &Policy::default(), &prefixes(), NOW);
        let rows = json["deployments"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name_and_version"], "metaboard-base/old");
        assert_eq!(rows[0]["verdict"], "candidate");
        assert_eq!(rows[0]["reason"], "superseded");
        assert_eq!(rows[0]["age_days"], serde_json::json!(100));
        assert_eq!(rows[1]["verdict"], "retained");
        assert_eq!(rows[1]["reason"], "tag-target");
    }

    #[test]
    fn the_json_report_separates_alias_rows_from_deployments() {
        let json = render_json(&mixed_report(), &Policy::default(), &prefixes(), NOW);
        let tags = json["tags"].as_array().unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0]["label"], "latest");
        assert_eq!(tags[0]["target_version"], "new");
        assert_eq!(tags[0]["dangling"], serde_json::json!(false));
    }

    #[test]
    fn a_dangling_alias_is_flagged_in_json() {
        let entries = vec![alias("metaboard-base", "latest", None)];
        let report = classify(&entries, &Policy::default(), NOW);
        let json = render_json(&report, &Policy::default(), &prefixes(), NOW);
        assert_eq!(json["tags"][0]["dangling"], serde_json::json!(true));
        assert_eq!(json["tags"][0]["target_version"], serde_json::Value::Null);
    }

    #[test]
    fn the_json_report_echoes_the_policy_it_was_run_under() {
        let policy = Policy {
            min_age_days: 7,
            pinned: ["metaboard-base/keep".to_string()].into_iter().collect(),
        };
        let json = render_json(&mixed_report(), &policy, &prefixes(), NOW);
        assert_eq!(json["policy"]["min_age_days"], serde_json::json!(7));
        assert_eq!(
            json["policy"]["name_prefixes"],
            serde_json::json!(["metaboard"])
        );
        assert_eq!(
            json["policy"]["pinned"],
            serde_json::json!(["metaboard-base/keep"])
        );
    }

    #[test]
    fn an_undated_row_reports_a_null_age_rather_than_a_number() {
        let entries = vec![deployment("metaboard-base", "undated", None)];
        let report = classify(&entries, &Policy::default(), NOW);
        let json = render_json(&report, &Policy::default(), &prefixes(), NOW);
        assert_eq!(json["deployments"][0]["age_days"], serde_json::Value::Null);
        assert_eq!(json["deployments"][0]["reason"], "unknown-age");
    }

    #[test]
    fn the_generated_timestamp_is_the_clock_reading_supplied() {
        let json = render_json(&mixed_report(), &Policy::default(), &prefixes(), NOW);
        assert_eq!(json["generated_at_ms"], serde_json::json!(NOW));
    }

    #[test]
    fn a_lone_newest_deployment_renders_the_newest_reason() {
        let entries = vec![deployment("metaboard-base", "only", Some(NOW))];
        let report = classify(&entries, &Policy::default(), NOW);
        let json = render_json(&report, &Policy::default(), &prefixes(), NOW);
        assert_eq!(json["deployments"][0]["reason"], "newest");
        let table = render_table(&report, &Policy::default(), &prefixes(), NOW);
        assert!(table.contains("retained: newest for this name"));
    }

    // ---------- candidates ----------

    #[test]
    fn the_candidate_list_is_one_name_and_version_per_line() {
        let entries = vec![
            deployment("metaboard-base", "a", Some(NOW - 300 * DAY)),
            deployment("metaboard-base", "b", Some(NOW - 200 * DAY)),
            deployment("metaboard-base", "c", Some(NOW)),
        ];
        let report = classify(&entries, &Policy::default(), NOW);
        assert_eq!(
            render_candidates(&report),
            "metaboard-base/a\nmetaboard-base/b"
        );
    }

    #[test]
    fn the_candidate_list_is_empty_when_nothing_is_superseded() {
        let entries = vec![deployment("metaboard-base", "only", Some(NOW))];
        let report = classify(&entries, &Policy::default(), NOW);
        assert_eq!(render_candidates(&report), "");
    }

    #[test]
    fn the_candidate_list_never_emits_a_runnable_delete_command() {
        // Reaping is human-dispatched. The list is identifiers, not commands:
        // it must not be pipeable into a shell.
        let output = render_candidates(&mixed_report());
        assert!(!output.contains("goldsky"));
        assert!(!output.contains("delete"));
        assert_eq!(output, "metaboard-base/old");
    }

    // ---------- table ----------

    #[test]
    fn the_table_states_the_usage_caveat() {
        let table = render_table(&mixed_report(), &Policy::default(), &prefixes(), NOW);
        assert!(table.contains(USAGE_CAVEAT));
    }

    #[test]
    fn the_table_counts_deployments_aliases_and_candidates() {
        let table = render_table(&mixed_report(), &Policy::default(), &prefixes(), NOW);
        assert!(
            table.contains("2 deployed version(s), 1 tag alias(es), 1 reaping candidate(s)."),
            "unexpected summary in:\n{table}"
        );
    }

    #[test]
    fn the_table_marks_the_candidate_and_the_retained_row_differently() {
        let table = render_table(&mixed_report(), &Policy::default(), &prefixes(), NOW);
        assert!(table.contains("metaboard-base/old"));
        assert!(table.contains("CANDIDATE: superseded, 100d old"));
        assert!(table.contains("retained: a tag resolves here"));
    }

    #[test]
    fn the_table_renders_the_alias_arrow() {
        let table = render_table(&mixed_report(), &Policy::default(), &prefixes(), NOW);
        assert!(table.contains("metaboard-base/latest -> metaboard-base/new"));
    }

    #[test]
    fn the_table_marks_a_dangling_alias() {
        let entries = vec![alias("metaboard-base", "latest", None)];
        let report = classify(&entries, &Policy::default(), NOW);
        let table = render_table(&report, &Policy::default(), &prefixes(), NOW);
        assert!(table.contains("[no target] (dangling)"));
    }

    #[test]
    fn the_table_says_so_when_nothing_matched() {
        let report = classify(&[], &Policy::default(), NOW);
        let table = render_table(&report, &Policy::default(), &prefixes(), NOW);
        assert!(table.contains("No deployed versions matched."));
        assert!(table.contains("0 deployed version(s), 0 tag alias(es), 0 reaping candidate(s)."));
    }

    #[test]
    fn the_table_reports_the_window_and_prefixes_it_ran_under() {
        let policy = Policy {
            min_age_days: 90,
            pinned: Default::default(),
        };
        let table = render_table(
            &mixed_report(),
            &policy,
            &[String::from("metaboard"), String::from("metadata")],
            NOW,
        );
        assert!(table.contains("metaboard, metadata"));
        assert!(table.contains("window: 90d"));
    }

    #[test]
    fn an_empty_prefix_list_renders_as_all() {
        let table = render_table(&mixed_report(), &Policy::default(), &[], NOW);
        assert!(table.contains("<all>"));
    }

    #[test]
    fn the_table_marks_a_paused_row() {
        let mut paused = deployment("metaboard-base", "only", Some(NOW));
        paused.paused = true;
        let report = classify(&[paused], &Policy::default(), NOW);
        let table = render_table(&report, &Policy::default(), &prefixes(), NOW);
        assert!(table.contains("[paused]"));
    }

    #[test]
    fn an_undated_row_shows_a_question_mark_for_its_age() {
        let entries = vec![deployment("metaboard-base", "undated", None)];
        let report = classify(&entries, &Policy::default(), NOW);
        let table = render_table(&report, &Policy::default(), &prefixes(), NOW);
        assert!(table.contains('?'));
        assert!(table.contains("age unknown"));
    }

    #[test]
    fn the_table_never_emits_a_runnable_delete_command() {
        let table = render_table(&mixed_report(), &Policy::default(), &prefixes(), NOW);
        assert!(!table.contains("subgraph delete"));
    }
}
