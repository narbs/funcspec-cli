use anyhow::{bail, Result};
use colored::Colorize;
use comfy_table::{Attribute, Cell, ContentArrangement, Table};
use funcspec_client::models::*;

use crate::context::client_and_project;
use crate::output::OutputFormat;

pub enum RunsCmd {
    List {
        page: u32,
        per: u32,
    },
    Create {
        type_of: Option<String>,
        status: Option<String>,
        score_below: Option<u32>,
        tag: Option<String>,
        item_ids: Vec<u64>,
        concurrency: Option<u32>,
        name: Option<String>,
        no_start: bool,
    },
    Show {
        run_id: u64,
        page: u32,
        per: u32,
    },
    Update {
        run_id: u64,
        name: String,
    },
    Start {
        run_id: u64,
    },
    Pause {
        run_id: u64,
    },
    Resume {
        run_id: u64,
    },
    Cancel {
        run_id: u64,
    },
    Delete {
        run_id: u64,
        yes: bool,
    },
    Watch {
        run_id: u64,
        fail_below: Option<f64>,
        interval: u64,
    },
}

pub fn build_command() -> clap::Command {
    clap::Command::new("run")
        .about("Manage audit runs")
        .arg_required_else_help(true)
        .subcommand(
            clap::Command::new("list")
                .about("List audit runs for the current project")
                .arg(
                    clap::Arg::new("page")
                        .long("page")
                        .value_parser(clap::value_parser!(u32))
                        .default_value("1")
                        .help("Page number"),
                )
                .arg(
                    clap::Arg::new("per")
                        .long("per")
                        .value_parser(clap::value_parser!(u32))
                        .default_value("20")
                        .help("Results per page"),
                ),
        )
        .subcommand(
            clap::Command::new("create")
                .about("Create (and optionally start) a new audit run")
                .arg(
                    clap::Arg::new("type")
                        .long("type")
                        .short('t')
                        .help("Filter by item type: func or tech"),
                )
                .arg(
                    clap::Arg::new("status")
                        .long("status")
                        .short('s')
                        .help("Filter by implementation status"),
                )
                .arg(
                    clap::Arg::new("score_below")
                        .long("score-below")
                        .value_parser(clap::value_parser!(u32))
                        .help("Include only items with coverage score below N"),
                )
                .arg(
                    clap::Arg::new("tag")
                        .long("tag")
                        .help("Filter by tag(s), comma-separated (OR logic)"),
                )
                .arg(
                    clap::Arg::new("item_ids")
                        .long("item-ids")
                        .num_args(1..)
                        .value_delimiter(',')
                        .value_parser(clap::value_parser!(u64))
                        .help("Explicit item IDs to include (comma-separated)"),
                )
                .arg(
                    clap::Arg::new("concurrency")
                        .long("concurrency")
                        .value_parser(clap::value_parser!(u32))
                        .help("Requested concurrency (clamped to server max)"),
                )
                .arg(
                    clap::Arg::new("name")
                        .long("name")
                        .short('n')
                        .help("Optional run name"),
                )
                .arg(
                    clap::Arg::new("no_start")
                        .long("no-start")
                        .action(clap::ArgAction::SetTrue)
                        .help("Create in draft state without starting"),
                ),
        )
        .subcommand(
            clap::Command::new("show")
                .about("Show a run's details and item results")
                .arg(
                    clap::Arg::new("run_id")
                        .required(true)
                        .value_parser(clap::value_parser!(u64))
                        .help("Run ID"),
                )
                .arg(
                    clap::Arg::new("page")
                        .long("page")
                        .value_parser(clap::value_parser!(u32))
                        .default_value("1")
                        .help("Page of items to show"),
                )
                .arg(
                    clap::Arg::new("per")
                        .long("per")
                        .value_parser(clap::value_parser!(u32))
                        .default_value("50")
                        .help("Items per page"),
                ),
        )
        .subcommand(
            clap::Command::new("update")
                .about("Update a draft run's name")
                .arg(
                    clap::Arg::new("run_id")
                        .required(true)
                        .value_parser(clap::value_parser!(u64))
                        .help("Run ID"),
                )
                .arg(
                    clap::Arg::new("name")
                        .long("name")
                        .short('n')
                        .required(true)
                        .help("New run name"),
                ),
        )
        .subcommand(
            clap::Command::new("start")
                .about("Start a draft run")
                .arg(
                    clap::Arg::new("run_id")
                        .required(true)
                        .value_parser(clap::value_parser!(u64))
                        .help("Run ID"),
                ),
        )
        .subcommand(
            clap::Command::new("pause")
                .about("Pause a running run")
                .arg(
                    clap::Arg::new("run_id")
                        .required(true)
                        .value_parser(clap::value_parser!(u64))
                        .help("Run ID"),
                ),
        )
        .subcommand(
            clap::Command::new("resume")
                .about("Resume a paused run")
                .arg(
                    clap::Arg::new("run_id")
                        .required(true)
                        .value_parser(clap::value_parser!(u64))
                        .help("Run ID"),
                ),
        )
        .subcommand(
            clap::Command::new("cancel")
                .about("Cancel a run (draft, running, or paused)")
                .arg(
                    clap::Arg::new("run_id")
                        .required(true)
                        .value_parser(clap::value_parser!(u64))
                        .help("Run ID"),
                ),
        )
        .subcommand(
            clap::Command::new("delete")
                .about("Delete a run (auto-cancels if running)")
                .arg(
                    clap::Arg::new("run_id")
                        .required(true)
                        .value_parser(clap::value_parser!(u64))
                        .help("Run ID"),
                )
                .arg(
                    clap::Arg::new("yes")
                        .long("yes")
                        .short('y')
                        .action(clap::ArgAction::SetTrue)
                        .help("Skip confirmation"),
                ),
        )
        .subcommand(
            clap::Command::new("watch")
                .about("Poll a run until it completes (or fails below a score threshold)")
                .arg(
                    clap::Arg::new("run_id")
                        .required(true)
                        .value_parser(clap::value_parser!(u64))
                        .help("Run ID"),
                )
                .arg(
                    clap::Arg::new("fail_below")
                        .long("fail-below")
                        .value_parser(clap::value_parser!(f64))
                        .help("Exit non-zero if avg_score_after drops below this value"),
                )
                .arg(
                    clap::Arg::new("interval")
                        .long("interval")
                        .value_parser(clap::value_parser!(u64))
                        .default_value("5")
                        .help("Poll interval in seconds"),
                ),
        )
}

pub async fn dispatch(matches: &clap::ArgMatches, format: OutputFormat) -> Result<()> {
    let cmd = match matches.subcommand() {
        Some(("list", m)) => RunsCmd::List {
            page: m.get_one::<u32>("page").copied().unwrap_or(1),
            per: m.get_one::<u32>("per").copied().unwrap_or(20),
        },
        Some(("create", m)) => RunsCmd::Create {
            type_of: m.get_one::<String>("type").map(|t| match t.as_str() {
                "func" | "functional" => "functional".to_string(),
                "tech" | "technical" => "technical".to_string(),
                other => other.to_string(),
            }),
            status: m.get_one::<String>("status").cloned(),
            score_below: m.get_one::<u32>("score_below").copied(),
            tag: m.get_one::<String>("tag").cloned(),
            item_ids: m
                .get_many::<u64>("item_ids")
                .map(|v| v.copied().collect())
                .unwrap_or_default(),
            concurrency: m.get_one::<u32>("concurrency").copied(),
            name: m.get_one::<String>("name").cloned(),
            no_start: m.get_flag("no_start"),
        },
        Some(("show", m)) => RunsCmd::Show {
            run_id: *m.get_one::<u64>("run_id").unwrap(),
            page: m.get_one::<u32>("page").copied().unwrap_or(1),
            per: m.get_one::<u32>("per").copied().unwrap_or(50),
        },
        Some(("update", m)) => RunsCmd::Update {
            run_id: *m.get_one::<u64>("run_id").unwrap(),
            name: m.get_one::<String>("name").unwrap().clone(),
        },
        Some(("start", m)) => RunsCmd::Start {
            run_id: *m.get_one::<u64>("run_id").unwrap(),
        },
        Some(("pause", m)) => RunsCmd::Pause {
            run_id: *m.get_one::<u64>("run_id").unwrap(),
        },
        Some(("resume", m)) => RunsCmd::Resume {
            run_id: *m.get_one::<u64>("run_id").unwrap(),
        },
        Some(("cancel", m)) => RunsCmd::Cancel {
            run_id: *m.get_one::<u64>("run_id").unwrap(),
        },
        Some(("delete", m)) => RunsCmd::Delete {
            run_id: *m.get_one::<u64>("run_id").unwrap(),
            yes: m.get_flag("yes"),
        },
        Some(("watch", m)) => RunsCmd::Watch {
            run_id: *m.get_one::<u64>("run_id").unwrap(),
            fail_below: m.get_one::<f64>("fail_below").copied(),
            interval: m.get_one::<u64>("interval").copied().unwrap_or(5),
        },
        _ => {
            build_command().print_help().ok();
            return Ok(());
        }
    };
    run(cmd, format).await
}

pub async fn run(cmd: RunsCmd, format: OutputFormat) -> Result<()> {
    match cmd {
        RunsCmd::List { page, per } => {
            let (client, project_id) = client_and_project().await?;
            let (runs, meta) = client.list_runs(project_id, page, per).await?;

            match format.resolve() {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&runs)?);
                }
                _ => {
                    if runs.is_empty() {
                        println!("No runs found.");
                        return Ok(());
                    }
                    let mut table = Table::new();
                    table.set_content_arrangement(ContentArrangement::Dynamic);
                    table.set_header(vec![
                        Cell::new("ID").add_attribute(Attribute::Bold),
                        Cell::new("Name").add_attribute(Attribute::Bold),
                        Cell::new("State").add_attribute(Attribute::Bold),
                        Cell::new("Total").add_attribute(Attribute::Bold),
                        Cell::new("Done").add_attribute(Attribute::Bold),
                        Cell::new("Avg Score").add_attribute(Attribute::Bold),
                        Cell::new("Created").add_attribute(Attribute::Bold),
                    ]);
                    for r in &runs {
                        table.add_row(vec![
                            Cell::new(r.id.to_string()),
                            Cell::new(r.name.as_deref().unwrap_or("-")),
                            Cell::new(state_colored(&r.state)),
                            Cell::new(r.stats.total.to_string()),
                            Cell::new(r.stats.done.to_string()),
                            Cell::new(
                                r.stats
                                    .avg_score_after
                                    .map(|s| format!("{s:.0}"))
                                    .unwrap_or_else(|| "-".to_string()),
                            ),
                            Cell::new(r.created_at.format("%Y-%m-%d %H:%M").to_string()),
                        ]);
                    }
                    println!("{table}");
                    if let Some(m) = meta {
                        println!("Page {}/{} ({} total)", m.page, m.total_pages.max(1), m.total);
                    }
                }
            }
            Ok(())
        }

        RunsCmd::Create {
            type_of,
            status,
            score_below,
            tag,
            item_ids,
            concurrency,
            name,
            no_start,
        } => {
            let (client, project_id) = client_and_project().await?;
            let params = CreateRunParams {
                type_of,
                status,
                score_below,
                tag,
                item_ids,
                concurrency,
                name,
                no_start: if no_start { Some(true) } else { None },
            };
            let audit_run = client.create_run(project_id, &params).await?;
            match format.resolve() {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&audit_run)?);
                }
                _ => {
                    print_run_summary(&audit_run);
                }
            }
            Ok(())
        }

        RunsCmd::Show { run_id, page, per } => {
            let (client, project_id) = client_and_project().await?;
            let audit_run = client.get_run(project_id, run_id, page, per).await?;

            match format.resolve() {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&audit_run)?);
                }
                _ => {
                    print_run_summary(&audit_run);
                    if !audit_run.items.is_empty() {
                        println!();
                        print_items_table(&audit_run.items);
                        if let (Some(p), Some(tp)) = (audit_run.page, audit_run.total_pages) {
                            if tp > 1 {
                                println!("Items page {p}/{tp}");
                            }
                        }
                    }
                }
            }
            Ok(())
        }

        RunsCmd::Update { run_id, name } => {
            let (client, project_id) = client_and_project().await?;
            let params = UpdateRunParams { name: Some(name) };
            let audit_run = client.update_run(project_id, run_id, &params).await?;
            match format.resolve() {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&audit_run)?);
                }
                _ => {
                    println!("Run {} updated: {}", audit_run.id, audit_run.name.as_deref().unwrap_or("-"));
                }
            }
            Ok(())
        }

        RunsCmd::Start { run_id } => {
            let (client, project_id) = client_and_project().await?;
            let audit_run = client.start_run(project_id, run_id).await?;
            match format.resolve() {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&audit_run)?);
                }
                _ => {
                    println!("Run {} started (state: {})", audit_run.id, state_colored(&audit_run.state));
                }
            }
            Ok(())
        }

        RunsCmd::Pause { run_id } => {
            let (client, project_id) = client_and_project().await?;
            let audit_run = client.pause_run(project_id, run_id).await?;
            match format.resolve() {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&audit_run)?);
                }
                _ => {
                    println!("Run {} paused.", audit_run.id);
                }
            }
            Ok(())
        }

        RunsCmd::Resume { run_id } => {
            let (client, project_id) = client_and_project().await?;
            let audit_run = client.resume_run(project_id, run_id).await?;
            match format.resolve() {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&audit_run)?);
                }
                _ => {
                    println!("Run {} resumed (state: {})", audit_run.id, state_colored(&audit_run.state));
                }
            }
            Ok(())
        }

        RunsCmd::Cancel { run_id } => {
            let (client, project_id) = client_and_project().await?;
            let audit_run = client.cancel_run(project_id, run_id).await?;
            match format.resolve() {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&audit_run)?);
                }
                _ => {
                    println!("Run {} cancelled.", audit_run.id);
                }
            }
            Ok(())
        }

        RunsCmd::Delete { run_id, yes } => {
            if !yes {
                eprint!("Delete run {run_id}? [y/N] ");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Aborted.");
                    return Ok(());
                }
            }
            let (client, project_id) = client_and_project().await?;
            client.delete_run(project_id, run_id).await?;
            println!("Run {run_id} deleted.");
            Ok(())
        }

        RunsCmd::Watch {
            run_id,
            fail_below,
            interval,
        } => {
            let (client, project_id) = client_and_project().await?;
            loop {
                let audit_run = client.get_run(project_id, run_id, 1, 1).await?;
                let s = &audit_run.stats;
                eprint!(
                    "\r[{}] {}/{} done  {} failed  {} pending  ",
                    state_colored(&audit_run.state),
                    s.done,
                    s.total,
                    s.failed + s.failed_permanently,
                    s.pending,
                );

                match audit_run.state.as_str() {
                    "completed" | "cancelled" => {
                        eprintln!();
                        match format.resolve() {
                            OutputFormat::Json => {
                                println!("{}", serde_json::to_string_pretty(&audit_run)?);
                            }
                            _ => {
                                print_run_summary(&audit_run);
                            }
                        }
                        if let Some(threshold) = fail_below {
                            if let Some(avg) = audit_run.stats.avg_score_after {
                                if avg < threshold {
                                    bail!(
                                        "avg_score_after {avg:.1} is below threshold {threshold:.1}"
                                    );
                                }
                            }
                        }
                        return Ok(());
                    }
                    _ => {}
                }
                tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            }
        }
    }
}

fn state_colored(state: &str) -> String {
    match state {
        "running" => state.green().to_string(),
        "completed" => state.cyan().to_string(),
        "cancelled" => state.yellow().to_string(),
        "failed" => state.red().to_string(),
        "paused" => state.yellow().to_string(),
        "draft" => state.dimmed().to_string(),
        other => other.to_string(),
    }
}

fn verdict_colored(verdict: &str) -> String {
    match verdict {
        "ready" => verdict.green().to_string(),
        "needs_refinement" => verdict.yellow().to_string(),
        "major_gaps" => verdict.red().to_string(),
        other => other.to_string(),
    }
}

fn print_run_summary(r: &AuditRun) {
    println!(
        "Run {} — {} [{}]",
        r.id.to_string().bold(),
        r.name.as_deref().unwrap_or("(unnamed)"),
        state_colored(&r.state)
    );
    let s = &r.stats;
    println!(
        "  Items: {} total  {} done  {} pending  {} failed  {} cancelled",
        s.total, s.done, s.pending, s.failed + s.failed_permanently, s.cancelled
    );
    if s.done > 0 {
        println!(
            "  Scores: before={} after={}  improved={} regressed={} unchanged={}",
            s.avg_score_before.map(|v| format!("{v:.1}")).unwrap_or_else(|| "-".to_string()),
            s.avg_score_after.map(|v| format!("{v:.1}")).unwrap_or_else(|| "-".to_string()),
            s.improved,
            s.regressed,
            s.unchanged,
        );
    }
    if let Some(started) = r.started_at {
        print!("  Started: {}", started.format("%Y-%m-%d %H:%M:%S UTC"));
        if let Some(completed) = r.completed_at {
            let elapsed = completed - started;
            println!("  Completed: {} ({:.0}s)", completed.format("%Y-%m-%d %H:%M:%S UTC"), elapsed.num_seconds());
        } else {
            println!();
        }
    }
}

fn print_items_table(items: &[AuditRunItem]) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("Permalink").add_attribute(Attribute::Bold),
        Cell::new("Title").add_attribute(Attribute::Bold),
        Cell::new("State").add_attribute(Attribute::Bold),
        Cell::new("Before").add_attribute(Attribute::Bold),
        Cell::new("After").add_attribute(Attribute::Bold),
        Cell::new("Δ").add_attribute(Attribute::Bold),
        Cell::new("Verdict").add_attribute(Attribute::Bold),
    ]);
    for i in items {
        let delta_str = match i.delta {
            Some(d) if d > 0.0 => format!("+{d:.0}").green().to_string(),
            Some(d) if d < 0.0 => format!("{d:.0}").red().to_string(),
            Some(_) => "0".dimmed().to_string(),
            None => "-".to_string(),
        };
        table.add_row(vec![
            Cell::new(i.permalink.as_deref().unwrap_or("-")),
            Cell::new(i.title.as_deref().unwrap_or("-")),
            Cell::new(state_colored(&i.state)),
            Cell::new(i.old_score.map(|s| format!("{s:.0}")).unwrap_or_else(|| "-".to_string())),
            Cell::new(i.new_score.map(|s| format!("{s:.0}")).unwrap_or_else(|| "-".to_string())),
            Cell::new(delta_str),
            Cell::new(i.verdict.as_deref().map(verdict_colored).unwrap_or_else(|| "-".to_string())),
        ]);
    }
    println!("{table}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_list_defaults() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["run", "list"])
            .expect("list should parse");
        let sub = m.subcommand_matches("list").unwrap();
        assert_eq!(sub.get_one::<u32>("page").copied(), Some(1));
        assert_eq!(sub.get_one::<u32>("per").copied(), Some(20));
    }

    #[test]
    fn dispatch_create_no_start_flag() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["run", "create", "--no-start", "--type", "func"])
            .expect("create should parse");
        let sub = m.subcommand_matches("create").unwrap();
        assert!(sub.get_flag("no_start"));
        assert_eq!(sub.get_one::<String>("type").map(|s| s.as_str()), Some("func"));
    }

    #[test]
    fn dispatch_watch_parses_fail_below() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["run", "watch", "42", "--fail-below", "60"])
            .expect("watch should parse");
        let sub = m.subcommand_matches("watch").unwrap();
        assert_eq!(sub.get_one::<u64>("run_id").copied(), Some(42));
        assert_eq!(sub.get_one::<f64>("fail_below").copied(), Some(60.0));
    }

    #[test]
    fn dispatch_delete_yes_flag() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["run", "delete", "7", "-y"])
            .expect("delete -y should parse");
        let sub = m.subcommand_matches("delete").unwrap();
        assert!(sub.get_flag("yes"));
        assert_eq!(sub.get_one::<u64>("run_id").copied(), Some(7));
    }
}
