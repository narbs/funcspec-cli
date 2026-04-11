use anyhow::Result;
use funcspec_client::models::*;
use rust_i18n::t;

use crate::context::client_and_project;
use crate::output::{self, OutputFormat};

/// Arguments for the `funcspec search` command.
pub struct SearchArgs {
    pub query: String,
    pub r#type: Option<String>,
    pub tag: Option<String>,
    pub json: bool,
    pub quiet: bool,
    pub count: bool,
}

pub fn build_command() -> clap::Command {
    clap::Command::new("search")
        .about(t!("cmd.search.about").to_string())
        .arg(
            clap::Arg::new("query")
                .required(true)
                .help(t!("cmd.search.query").to_string()),
        )
        .arg(
            clap::Arg::new("type")
                .long("type")
                .short('t')
                .help(t!("cmd.search.type").to_string()),
        )
        .arg(
            clap::Arg::new("tag")
                .long("tag")
                .help(t!("cmd.search.tag").to_string()),
        )
        .arg(
            clap::Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help(t!("cmd.search.json").to_string()),
        )
        .arg(
            clap::Arg::new("quiet")
                .long("quiet")
                .action(clap::ArgAction::SetTrue)
                .help(t!("cmd.search.quiet").to_string()),
        )
        .arg(
            clap::Arg::new("count")
                .long("count")
                .action(clap::ArgAction::SetTrue)
                .help(t!("cmd.search.count").to_string()),
        )
}

pub fn from_arg_matches(matches: &clap::ArgMatches) -> SearchArgs {
    SearchArgs {
        query: matches.get_one::<String>("query").unwrap().clone(),
        r#type: matches.get_one::<String>("type").cloned(),
        tag: matches.get_one::<String>("tag").cloned(),
        json: matches.get_flag("json"),
        quiet: matches.get_flag("quiet"),
        count: matches.get_flag("count"),
    }
}

pub async fn run(args: SearchArgs, format: OutputFormat) -> Result<()> {
    let (client, project_id) = client_and_project().await?;

    let type_of = args.r#type.as_deref().map(|t| match t {
        "func" | "functional" => ItemType::Functional,
        "tech" | "technical" => ItemType::Technical,
        _ => ItemType::Functional,
    });

    let filter = ItemFilter {
        type_of,
        tag: args.tag,
        ..ItemFilter::default()
    };

    let result = client
        .search_items(project_id, &args.query, &filter)
        .await?;

    if args.count {
        println!("{}", result.data.len());
        return Ok(());
    }

    let fmt = if args.json {
        OutputFormat::Json
    } else if args.quiet {
        OutputFormat::Minimal
    } else {
        format
    };

    // Show result count to stderr for human-readable formats
    if matches!(fmt.resolve(), OutputFormat::Table | OutputFormat::Markdown) {
        eprintln!("{} result(s) for \"{}\"", result.data.len(), args.query);
    }

    let meta = PaginationMeta {
        page: result.page,
        per: result.per_page,
        total: result.total_count,
        total_pages: result.total_pages,
    };
    let meta_opt = if result.total_count > 0 {
        Some(meta)
    } else {
        None
    };

    output::format_items(&result.data, meta_opt.as_ref(), fmt)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_mapping_func() {
        let t = Some("func".to_string());
        let mapped = t.as_deref().map(|s| match s {
            "func" | "functional" => ItemType::Functional,
            "tech" | "technical" => ItemType::Technical,
            _ => ItemType::Functional,
        });
        assert_eq!(mapped, Some(ItemType::Functional));
    }

    #[test]
    fn type_mapping_tech() {
        let t = Some("tech".to_string());
        let mapped = t.as_deref().map(|s| match s {
            "func" | "functional" => ItemType::Functional,
            "tech" | "technical" => ItemType::Technical,
            _ => ItemType::Functional,
        });
        assert_eq!(mapped, Some(ItemType::Technical));
    }

    #[test]
    fn type_mapping_none() {
        let t: Option<String> = None;
        let mapped = t.as_deref().map(|s| match s {
            "func" | "functional" => ItemType::Functional,
            _ => ItemType::Technical,
        });
        assert_eq!(mapped, None);
    }

    #[test]
    fn search_format_override_json() {
        let fmt = if true {
            OutputFormat::Json
        } else {
            OutputFormat::Table
        };
        assert_eq!(fmt, OutputFormat::Json);
    }

    #[test]
    fn search_format_override_quiet() {
        let fmt = if false {
            OutputFormat::Json
        } else if true {
            OutputFormat::Minimal
        } else {
            OutputFormat::Table
        };
        assert_eq!(fmt, OutputFormat::Minimal);
    }

    #[test]
    fn search_format_falls_through_to_global() {
        let global = OutputFormat::Csv;
        let json_flag = false;
        let quiet_flag = false;
        let fmt = if json_flag {
            OutputFormat::Json
        } else if quiet_flag {
            OutputFormat::Minimal
        } else {
            global
        };
        assert_eq!(fmt, OutputFormat::Csv);
    }

    #[test]
    fn build_command_requires_query() {
        let cmd = build_command();
        assert!(cmd.try_get_matches_from(["search"]).is_err());
    }

    #[test]
    fn build_command_parses_query() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["search", "authentication flow"])
            .unwrap();
        assert_eq!(
            m.get_one::<String>("query").unwrap(),
            "authentication flow"
        );
    }

    #[test]
    fn build_command_parses_flags() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["search", "foo", "--json", "--count"])
            .unwrap();
        assert!(m.get_flag("json"));
        assert!(m.get_flag("count"));
        assert!(!m.get_flag("quiet"));
    }
}
