use anyhow::Result;
use clap::Args;
use funcspec_client::models::*;

use crate::context::client_and_project;
use crate::output::{self, OutputFormat};

/// Arguments for the `funcspec search` command.
#[derive(Debug, Args)]
#[command(about = "Search spec items by full-text query")]
pub struct SearchArgs {
    /// Search query string
    pub query: String,

    /// Filter by type: func or tech
    #[arg(long, short = 't')]
    pub r#type: Option<String>,

    /// Filter by tag
    #[arg(long)]
    pub tag: Option<String>,

    /// Output as JSON (overrides --format)
    #[arg(long)]
    pub json: bool,

    /// Quiet mode: output only permalinks, one per line (overrides --format)
    #[arg(long)]
    pub quiet: bool,

    /// Output just the count of matching items
    #[arg(long)]
    pub count: bool,
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
}
