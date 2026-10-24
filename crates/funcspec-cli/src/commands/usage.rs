use anyhow::{Context, Result};

use crate::context::client_and_project;
use crate::output::{self, OutputFormat};
use funcspec_client::models::UsageFilter;

pub fn build_command() -> clap::Command {
    clap::Command::new("usage")
        .about("LLM usage summary and log for a project")
        .arg_required_else_help(true)
        .subcommand(
            clap::Command::new("summary")
                .about("Show LLM usage summary for the current month (or a specific year/month)")
                .arg(
                    clap::Arg::new("year")
                        .long("year")
                        .short('Y')
                        .value_name("N")
                        .value_parser(clap::value_parser!(i32))
                        .help("Filter to a specific year (e.g. 2026)"),
                )
                .arg(
                    clap::Arg::new("month")
                        .long("month")
                        .short('m')
                        .value_name("N")
                        .value_parser(clap::value_parser!(u32))
                        .help("Filter to a specific month 1-12 (e.g. 10)"),
                )
                .arg(
                    clap::Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Print raw JSON instead of human-readable output"),
                ),
        )
        .subcommand(
            clap::Command::new("logs")
                .about("Show LLM usage log entries for a project")
                .arg(
                    clap::Arg::new("year")
                        .long("year")
                        .short('Y')
                        .value_name("N")
                        .value_parser(clap::value_parser!(i32))
                        .help("Filter to a specific year (e.g. 2026)"),
                )
                .arg(
                    clap::Arg::new("month")
                        .long("month")
                        .short('m')
                        .value_name("N")
                        .value_parser(clap::value_parser!(u32))
                        .help("Filter to a specific month 1-12 (e.g. 10)"),
                )
                .arg(
                    clap::Arg::new("action-type")
                        .long("action-type")
                        .short('a')
                        .value_name("TYPE")
                        .help("Filter to a specific action type (e.g. review)"),
                )
                .arg(
                    clap::Arg::new("page")
                        .long("page")
                        .value_name("N")
                        .value_parser(clap::value_parser!(u32))
                        .help("Page number (default 1)"),
                )
                .arg(
                    clap::Arg::new("per")
                        .long("per")
                        .value_name("N")
                        .value_parser(clap::value_parser!(u32))
                        .help("Entries per page (default 25, max 100)"),
                )
                .arg(
                    clap::Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Print raw JSON instead of human-readable output"),
                ),
        )
}

pub async fn dispatch(matches: &clap::ArgMatches, format: OutputFormat) -> Result<()> {
    match matches.subcommand() {
        Some(("summary", sub)) => run_summary(sub, format).await,
        Some(("logs", sub)) => run_logs(sub, format).await,
        _ => {
            build_command().print_help().ok();
            Ok(())
        }
    }
}

async fn run_summary(matches: &clap::ArgMatches, _format: OutputFormat) -> Result<()> {
    let year = matches.get_one::<i32>("year").copied();
    let month = matches.get_one::<u32>("month").copied();
    let json = matches.get_flag("json");

    let (client, project_id) = client_and_project()
        .await
        .context("Not logged in or no project set. Run `funcspec auth login` and use -p <slug>.")?;

    let stats = client
        .get_usage_stats(project_id, year, month)
        .await
        .context("Failed to fetch usage summary")?;

    if json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
    } else {
        output::format_usage_stats(&stats);
    }

    Ok(())
}

async fn run_logs(matches: &clap::ArgMatches, _format: OutputFormat) -> Result<()> {
    let year = matches.get_one::<i32>("year").copied();
    let month = matches.get_one::<u32>("month").copied();
    let action_type = matches.get_one::<String>("action-type").cloned();
    let page = matches.get_one::<u32>("page").copied();
    let per = matches.get_one::<u32>("per").copied();
    let json = matches.get_flag("json");

    let (client, project_id) = client_and_project()
        .await
        .context("Not logged in or no project set. Run `funcspec auth login` and use -p <slug>.")?;

    let filter = UsageFilter {
        year,
        month,
        action_type,
        page,
        per,
    };

    let paged = client
        .get_usage_logs(project_id, &filter)
        .await
        .context("Failed to fetch usage logs")?;

    if json {
        println!("{}", serde_json::to_string_pretty(&paged.data)?);
    } else {
        output::format_usage_logs(&paged.data, paged.total_count, paged.page, paged.per_page);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_command_has_summary_and_logs() {
        let cmd = build_command();
        let sub_names: Vec<&str> = cmd.get_subcommands().map(|s| s.get_name()).collect();
        assert!(sub_names.contains(&"summary"), "missing summary subcommand");
        assert!(sub_names.contains(&"logs"), "missing logs subcommand");
    }

    #[test]
    fn summary_parses_year_month_json() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from([
                "usage", "summary", "--year", "2026", "--month", "10", "--json",
            ])
            .unwrap();
        let sub = m.subcommand_matches("summary").unwrap();
        assert_eq!(sub.get_one::<i32>("year").copied(), Some(2026));
        assert_eq!(sub.get_one::<u32>("month").copied(), Some(10));
        assert!(sub.get_flag("json"));
    }

    #[test]
    fn logs_parses_all_flags() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from([
                "usage",
                "logs",
                "--year",
                "2026",
                "--month",
                "10",
                "--action-type",
                "review",
                "--page",
                "2",
                "--per",
                "50",
                "--json",
            ])
            .unwrap();
        let sub = m.subcommand_matches("logs").unwrap();
        assert_eq!(sub.get_one::<i32>("year").copied(), Some(2026));
        assert_eq!(sub.get_one::<u32>("month").copied(), Some(10));
        assert_eq!(
            sub.get_one::<String>("action-type").map(String::as_str),
            Some("review")
        );
        assert_eq!(sub.get_one::<u32>("page").copied(), Some(2));
        assert_eq!(sub.get_one::<u32>("per").copied(), Some(50));
        assert!(sub.get_flag("json"));
    }
}
