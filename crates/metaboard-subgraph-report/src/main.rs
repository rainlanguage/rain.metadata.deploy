use anyhow::{Context, Result};
use clap::Parser;
use metaboard_subgraph_report::classify::{Policy, classify, filter_by_name_prefixes};
use metaboard_subgraph_report::goldsky::{DEFAULT_API_HOST, GoldskyClient};
use metaboard_subgraph_report::report::{render_candidates, render_json, render_table};
use std::time::{SystemTime, UNIX_EPOCH};

/// Environment variable carrying the Goldsky API token. CI surfaces the
/// `CI_GOLDSKY_TOKEN` secret under this name, matching the rest of the org.
const TOKEN_ENV: &str = "GOLDSKY_TOKEN";

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum Format {
    /// Aligned human table with a reason per row.
    Table,
    /// Machine-readable document.
    Json,
    /// Candidate `name/version` identifiers, one per line.
    Candidates,
}

#[derive(Debug, Parser)]
#[command(
    name = "metaboard-subgraph-report",
    about = "Report deployed metaboard subgraphs on Goldsky and which versions are superseded.",
    long_about = "Enumerates every subgraph deployed on Goldsky whose name matches the given \
prefix, and classifies each deployed version as retained or as a reaping candidate.\n\n\
Goldsky exposes no per-subgraph usage metrics, so candidacy is SUPERSESSION, not measured \
disuse: a candidate is a version replaced by a newer version of the same name, that no tag \
resolves to, that was not pinned, and that is older than the age window.\n\n\
This tool never deletes anything. It reports. Reaping is human-dispatched."
)]
struct Args {
    /// Goldsky API host.
    #[arg(long, default_value = DEFAULT_API_HOST)]
    api_host: String,

    /// Only consider subgraphs whose name starts with this. Repeatable.
    ///
    /// The current deploy rule produces `metaboard-<network>`. Older live
    /// deploys use other stems, so pass this more than once (or pass
    /// `--name-prefix ""`) to widen the sweep.
    #[arg(long = "name-prefix", default_values_t = [String::from("metaboard")])]
    name_prefix: Vec<String>,

    /// A superseded version must be at least this old to be a candidate.
    #[arg(long, default_value_t = 30)]
    min_age_days: i64,

    /// Never treat this `name/version` as a candidate. Repeatable.
    #[arg(long = "keep", value_name = "NAME/VERSION")]
    keep: Vec<String>,

    #[arg(long, value_enum, default_value_t = Format::Table)]
    format: Format,
}

fn now_ms() -> Result<i64> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the unix epoch")?
        .as_millis();
    i64::try_from(millis).context("system clock is beyond the representable range")
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let token = std::env::var(TOKEN_ENV).unwrap_or_default();
    let client = GoldskyClient::new(&args.api_host, &token).with_context(|| {
        format!("set {TOKEN_ENV} to a Goldsky API token with access to the project")
    })?;

    let entries = client
        .list_subgraphs()
        .await
        .context("could not list subgraphs from Goldsky")?;
    let matched = filter_by_name_prefixes(&entries, &args.name_prefix);

    let policy = Policy {
        min_age_days: args.min_age_days,
        pinned: args.keep.iter().cloned().collect(),
    };
    let now = now_ms()?;
    let report = classify(&matched, &policy, now);

    match args.format {
        Format::Table => print!("{}", render_table(&report, &policy, &args.name_prefix, now)),
        Format::Json => println!(
            "{}",
            serde_json::to_string_pretty(&render_json(&report, &policy, &args.name_prefix, now))?
        ),
        Format::Candidates => {
            let candidates = render_candidates(&report);
            if !candidates.is_empty() {
                println!("{candidates}");
            }
        }
    }

    Ok(())
}
