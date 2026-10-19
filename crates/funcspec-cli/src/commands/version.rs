use anyhow::Result;
use colored::Colorize;
use funcspec_client::models::ReleaseVersion;
use rust_i18n::t;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn build_command() -> clap::Command {
    clap::Command::new("version")
        .about(t!("cmd.version.about").to_string())
        .subcommand(
            clap::Command::new("assign")
                .about("Add a spec item to a release version (by semver e.g. 0.2.0, or id)")
                .arg(
                    clap::Arg::new("release_version")
                        .required(true)
                        .help("Target release version: semver (0.2.0) or id"),
                )
                .arg(
                    clap::Arg::new("item")
                        .long("item")
                        .num_args(1..)
                        .value_delimiter(',')
                        .required(true)
                        .help(
                            "Spec item permalink(s) or id(s) to add (comma-separated, e.g. T-414)",
                        ),
                ),
        )
        .subcommand(
            clap::Command::new("unassign")
                .about("Remove a spec item from a release version (by semver or id)")
                .arg(
                    clap::Arg::new("release_version")
                        .required(true)
                        .help("Release version: semver (0.2.0) or id"),
                )
                .arg(
                    clap::Arg::new("item")
                        .long("item")
                        .required(true)
                        .help("Spec item permalink or id to remove (e.g. T-414)"),
                ),
        )
        .subcommand(
            clap::Command::new("list")
                .about("List all release versions with item counts")
                .arg(
                    clap::Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            clap::Command::new("show")
                .about("Show a release version + member items with a state/status rollup")
                .arg(
                    clap::Arg::new("release_version")
                        .required(true)
                        .help("Release version: semver (0.1.0) or id"),
                )
                .arg(
                    clap::Arg::new("deprecated")
                        .long("deprecated")
                        .action(clap::ArgAction::SetTrue)
                        .help("Include out-of-scope (deprecated) member items"),
                )
                .arg(
                    clap::Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
}

pub async fn dispatch(
    matches: &clap::ArgMatches,
    _format: crate::output::OutputFormat,
) -> Result<()> {
    match matches.subcommand() {
        Some(("assign", m)) => {
            let version = m.get_one::<String>("release_version").unwrap().clone();
            let items: Vec<String> = m
                .get_many::<String>("item")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            let (client, project_id) = crate::context::client_and_project().await?;
            client
                .add_items_to_version(project_id, &version, &items)
                .await?;
            println!("Added {} to release version {}", items.join(", "), version);
            Ok(())
        }
        Some(("unassign", m)) => {
            let version = m.get_one::<String>("release_version").unwrap().clone();
            let item = m.get_one::<String>("item").unwrap().clone();
            let (client, project_id) = crate::context::client_and_project().await?;
            client
                .remove_item_from_version(project_id, &version, &item)
                .await?;
            println!("Removed {} from release version {}", item, version);
            Ok(())
        }
        Some(("list", m)) => {
            let json = m.get_flag("json");
            let (client, project_id) = crate::context::client_and_project().await?;
            let versions = client.list_versions(project_id).await?;
            print_version_list(&versions, json);
            Ok(())
        }
        Some(("show", m)) => {
            let version = m.get_one::<String>("release_version").unwrap().clone();
            let deprecated = m.get_flag("deprecated");
            let json = m.get_flag("json");
            let (client, project_id) = crate::context::client_and_project().await?;
            let v = client.get_version(project_id, &version, deprecated).await?;
            print_version_show(&v, json);
            Ok(())
        }
        _ => run(),
    }
}

/// Order for rollup display.
const STATES: [&str; 4] = ["inbox", "draft", "accepted", "integrated"];
const IMPLS: [&str; 5] = [
    "not_started",
    "in_progress",
    "implemented",
    "verified",
    "released",
];

fn print_version_list(versions: &[ReleaseVersion], json: bool) {
    if json {
        let rows: Vec<serde_json::Value> = versions
            .iter()
            .map(|v| {
                let a = &v.attributes;
                serde_json::json!({
                    "id": v.id,
                    "semver": a.semver,
                    "name": a.name,
                    "lifecycle_state": a.lifecycle_state,
                    "branch": a.branch,
                    "item_count": a.item_count,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&rows).unwrap_or_default()
        );
        return;
    }
    if versions.is_empty() {
        println!("No release versions.");
        return;
    }
    for v in versions {
        let a = &v.attributes;
        println!(
            "  {:10} {:12} items={:<4} branch={} {}",
            a.semver.cyan().bold(),
            a.lifecycle_state.as_deref().unwrap_or("-"),
            a.item_count.unwrap_or(0),
            a.branch.as_deref().unwrap_or("-"),
            a.name.as_deref().unwrap_or(""),
        );
    }
}

fn print_version_show(v: &ReleaseVersion, json: bool) {
    let a = &v.attributes;
    let items = &a.items;
    let total = items.len();

    // Rollups
    let mut by_state: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut by_impl: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for it in items {
        *by_state.entry(it.state.as_str()).or_insert(0) += 1;
        *by_impl
            .entry(it.implementation_status.as_str())
            .or_insert(0) += 1;
    }
    // Implementation %: implemented + verified + released, over total.
    let built = items
        .iter()
        .filter(|i| {
            matches!(
                i.implementation_status.as_str(),
                "implemented" | "verified" | "released"
            )
        })
        .count();
    let pct = if total > 0 {
        (built as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    if json {
        let by_state_json: serde_json::Map<String, serde_json::Value> = STATES
            .iter()
            .map(|k| {
                (
                    (*k).to_string(),
                    serde_json::json!(by_state.get(*k).copied().unwrap_or(0)),
                )
            })
            .collect();
        let by_impl_json: serde_json::Map<String, serde_json::Value> = IMPLS
            .iter()
            .map(|k| {
                (
                    (*k).to_string(),
                    serde_json::json!(by_impl.get(*k).copied().unwrap_or(0)),
                )
            })
            .collect();
        let out = serde_json::json!({
            "id": v.id,
            "semver": a.semver,
            "name": a.name,
            "lifecycle_state": a.lifecycle_state,
            "branch": a.branch,
            "item_count": total,
            "implementation_pct": (pct * 10.0).round() / 10.0,
            "by_state": by_state_json,
            "by_implementation": by_impl_json,
            "items": items.iter().map(|i| serde_json::json!({
                "permalink": i.permalink,
                "type_of": i.type_of,
                "state": i.state,
                "implementation_status": i.implementation_status,
                "deprecated": i.deprecated,
                "title": i.title,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
        return;
    }

    println!(
        "{} {}  ({} items, {:.1}% implemented)",
        "Release".bold(),
        a.semver.cyan().bold(),
        total,
        pct
    );
    if let Some(n) = a.name.as_deref().filter(|s| !s.is_empty()) {
        println!("  {} {}", "name:".bold(), n);
    }
    println!(
        "  {} {}   {} {}",
        "lifecycle:".bold(),
        a.lifecycle_state.as_deref().unwrap_or("-"),
        "branch:".bold(),
        a.branch.as_deref().unwrap_or("-"),
    );
    print!("  {} ", "by state:".bold());
    for st in STATES {
        print!("{}={} ", st, by_state.get(st).copied().unwrap_or(0));
    }
    println!();
    print!("  {} ", "by impl: ".bold());
    for im in IMPLS {
        print!("{}={} ", im, by_impl.get(im).copied().unwrap_or(0));
    }
    println!();
}
const PKG_NAME: &str = env!("CARGO_PKG_NAME");

pub fn run() -> Result<()> {
    println!("{PKG_NAME} {VERSION}");
    println!("Build: Rust {}", rustc_version());
    if let Some(host) = configured_host() {
        println!("Host:  {host}");
    } else {
        println!("Host:  (not configured — run `funcspec auth login`)");
    }
    Ok(())
}

fn configured_host() -> Option<String> {
    // Check env var first
    if let Ok(h) = std::env::var("FUNCSPEC_HOST") {
        return Some(h);
    }
    // Try loading from config
    crate::config::Config::load()
        .ok()
        .and_then(|c| c.active_profile())
        .map(|p| p.host)
}

fn rustc_version() -> String {
    // Captured at build time via RUSTC_VERSION env var, if set
    option_env!("RUSTC_VERSION")
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_string_is_semver() {
        // Just verify it's set and non-empty
        assert!(!VERSION.is_empty());
        // Should look like major.minor.patch
        let parts: Vec<&str> = VERSION.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn version_run_succeeds() {
        // Should not panic
        run().unwrap();
    }
}
