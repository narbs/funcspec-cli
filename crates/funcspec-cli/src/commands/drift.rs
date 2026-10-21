use anyhow::Result;
use colored::Colorize;
use funcspec_client::models::{ImplementationStatus, ItemFilter, ItemType, SpecItem};

use crate::context::client_and_project;
use crate::output::OutputFormat;

/// Arguments for the `funcspec drift` command.
pub struct DriftArgs {
    pub type_of: Option<String>,
    pub state: Option<String>,
    pub status: Option<String>,
    pub release_version: Option<String>,
    pub json: bool,
    pub count: bool,
}

pub fn build_command() -> clap::Command {
    clap::Command::new("drift")
        .about("Report items where workflow State and implementation Status disagree")
        .long_about(
            "List every item whose (workflow state, implementation status) pair \
             violates the coupling policy: built code sitting in an early workflow \
             state, released-but-not-integrated, or integrated-but-not-started. \
             Read-only; exits non-zero when drift is found (usable as a CI gate).",
        )
        .arg(
            clap::Arg::new("type")
                .short('t')
                .long("type")
                .value_name("type")
                .help("Filter by type: func or tech"),
        )
        .arg(
            clap::Arg::new("state")
                .long("state")
                .value_name("state")
                .help("Filter by workflow state: inbox, draft, accepted, integrated"),
        )
        .arg(
            clap::Arg::new("status")
                .short('s')
                .long("status")
                .value_name("status")
                .help("Filter by implementation status"),
        )
        .arg(
            clap::Arg::new("release_version")
                .long("release-version")
                .value_name("semver|id")
                .help("Scope drift to a release version's members (semver 0.1.0 or id)"),
        )
        .arg(
            clap::Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Output as JSON (overrides --format)"),
        )
        .arg(
            clap::Arg::new("count")
                .long("count")
                .action(clap::ArgAction::SetTrue)
                .help("Output only the count of drifting items"),
        )
}

pub fn from_arg_matches(matches: &clap::ArgMatches) -> DriftArgs {
    DriftArgs {
        type_of: matches.get_one::<String>("type").cloned(),
        state: matches.get_one::<String>("state").cloned(),
        status: matches.get_one::<String>("status").cloned(),
        release_version: matches.get_one::<String>("release_version").cloned(),
        json: matches.get_flag("json"),
        count: matches.get_flag("count"),
    }
}

// Rank the two state machines so we can compare "how far along" an item is.
// Workflow:  inbox(0) -> draft(1) -> accepted(2) -> integrated(3)
// Impl:      not_started(0) -> in_progress(1) -> implemented(2) -> verified(3) -> released(4)
fn state_rank(s: &str) -> i32 {
    match s {
        "inbox" => 0,
        "draft" => 1,
        "accepted" => 2,
        "integrated" => 3,
        _ => -1,
    }
}

fn impl_rank(s: &ImplementationStatus) -> i32 {
    match s {
        ImplementationStatus::NotStarted => 0,
        ImplementationStatus::InProgress => 1,
        ImplementationStatus::Implemented => 2,
        ImplementationStatus::Verified => 3,
        ImplementationStatus::Released => 4,
    }
}

fn impl_str(s: &ImplementationStatus) -> &'static str {
    match s {
        ImplementationStatus::NotStarted => "not_started",
        ImplementationStatus::InProgress => "in_progress",
        ImplementationStatus::Implemented => "implemented",
        ImplementationStatus::Verified => "verified",
        ImplementationStatus::Released => "released",
    }
}

// The minimal workflow-state transition path a -> b, stepping the machine.
// VALID_TRANSITIONS is a simple linear chain for the forward direction.
fn state_path(from: &str, to: &str) -> Vec<String> {
    let chain = ["inbox", "draft", "accepted", "integrated"];
    let fi = chain.iter().position(|s| *s == from);
    let ti = chain.iter().position(|s| *s == to);
    match (fi, ti) {
        (Some(fi), Some(ti)) if ti > fi => {
            chain[fi + 1..=ti].iter().map(|s| s.to_string()).collect()
        }
        (Some(fi), Some(ti)) if ti < fi => {
            chain[ti..fi].iter().rev().map(|s| s.to_string()).collect()
        }
        _ => Vec::new(),
    }
}

struct Drift {
    item: SpecItem,
    violation: &'static str,
    suggested_state: &'static str,
}

/// Evaluate the coupling policy for one item. Returns Some(Drift) when it
/// violates the policy, None when state and status are consistent.
fn evaluate(item: &SpecItem) -> Option<(&'static str, &'static str)> {
    let st = item.attributes.state.as_str();
    let im = &item.attributes.implementation_status;
    let sr = state_rank(st);
    let ir = impl_rank(im);

    // 1. Built (implemented/verified/released) but workflow state < accepted.
    if ir >= impl_rank(&ImplementationStatus::Implemented) && sr < state_rank("accepted") {
        return Some(("built-but-not-accepted", "accepted"));
    }
    // 2. Released but not integrated.
    if ir == impl_rank(&ImplementationStatus::Released) && st != "integrated" {
        return Some(("released-but-not-integrated", "integrated"));
    }
    // 3. Reverse drift: integrated workflow but implementation never started.
    if st == "integrated" && ir == impl_rank(&ImplementationStatus::NotStarted) {
        return Some(("integrated-but-not-started", "accepted"));
    }
    None
}

pub async fn run(args: DriftArgs, format: OutputFormat) -> Result<()> {
    let (client, project_id) = client_and_project().await?;

    let mut filter = ItemFilter::default();
    if let Some(ref t) = args.type_of {
        filter.type_of = match t.as_str() {
            "func" | "functional" => Some(ItemType::Functional),
            "tech" | "technical" => Some(ItemType::Technical),
            other => anyhow::bail!("Invalid type '{other}'. Use func or tech."),
        };
    }
    // Scope to a release version's members when requested.
    filter.release_version = args.release_version.clone();

    let items = client.list_all_items(project_id, &filter).await?;

    // Apply client-side state/status filters (not all are server filters).
    let drifts: Vec<Drift> = items
        .into_iter()
        .filter(|it| {
            args.state
                .as_deref()
                .map(|s| it.attributes.state == s)
                .unwrap_or(true)
        })
        .filter(|it| {
            args.status
                .as_deref()
                .map(|s| impl_str(&it.attributes.implementation_status) == s)
                .unwrap_or(true)
        })
        .filter_map(|it| {
            evaluate(&it).map(|(violation, suggested_state)| Drift {
                item: it,
                violation,
                suggested_state,
            })
        })
        .collect();

    if args.count {
        println!("{}", drifts.len());
        if !drifts.is_empty() {
            std::process::exit(1);
        }
        return Ok(());
    }

    let use_json = args.json || matches!(format, OutputFormat::Json);
    if use_json {
        let rows: Vec<serde_json::Value> = drifts
            .iter()
            .map(|d| {
                serde_json::json!({
                    "permalink": d.item.attributes.permalink,
                    "type": match d.item.attributes.type_of { ItemType::Functional => "functional", ItemType::Technical => "technical" },
                    "state": d.item.attributes.state,
                    "implementation_status": impl_str(&d.item.attributes.implementation_status),
                    "violation": d.violation,
                    "title": d.item.attributes.title,
                    "suggested_state_transitions": state_path(&d.item.attributes.state, d.suggested_state),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else if drifts.is_empty() {
        println!("{}", "No State/Status drift found.".green());
    } else {
        println!(
            "{}",
            format!("{} item(s) with State/Status drift:", drifts.len())
                .yellow()
                .bold()
        );
        for d in &drifts {
            let fix = state_path(&d.item.attributes.state, d.suggested_state);
            let fix_str = if fix.is_empty() {
                String::new()
            } else {
                format!("  -> step state: {}", fix.join(" -> "))
            };
            println!(
                "  {:7} {:9} impl={:12} [{}]{}  {}",
                d.item.attributes.permalink,
                d.item.attributes.state,
                impl_str(&d.item.attributes.implementation_status),
                d.violation.red(),
                fix_str,
                d.item.attributes.title.chars().take(60).collect::<String>(),
            );
        }
    }

    if !drifts.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}
