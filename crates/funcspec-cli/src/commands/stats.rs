use anyhow::{bail, Context, Result};
use clap::Args;

use crate::context::client_and_config;
use crate::output::{self, OutputFormat};

/// Arguments for the `funcspec stats` command.
#[derive(Debug, Args)]
#[command(about = "Show project stats and dashboard")]
pub struct StatsArgs {
    /// Output as JSON (machine-readable)
    #[arg(long)]
    pub json: bool,

    /// Focus on LLM usage stats
    #[arg(long)]
    pub usage: bool,

    /// Month for usage stats in YYYY-MM format (e.g. 2026-03)
    #[arg(long, value_name = "YYYY-MM")]
    pub month: Option<String>,
}

pub async fn run(args: StatsArgs, _format: OutputFormat) -> Result<()> {
    // Validate month format early
    if let Some(ref m) = args.month
        && !is_valid_month(m)
    {
        bail!(
            "Invalid month format '{}'. Use YYYY-MM (e.g., 2026-03)",
            m
        );
    }

    let (client, config) = client_and_config()?;
    let profile = config
        .active_profile()
        .context("Not logged in. Run `funcspec auth login` to connect.")?;
    let project_slug = profile
        .default_project
        .as_deref()
        .context("No default project set. Run `funcspec projects set-default <slug>`.")?;

    let project = client
        .get_project(project_slug)
        .await
        .with_context(|| format!("Project '{project_slug}' not found"))?;

    // --usage only
    if args.usage && !args.json {
        let usage = client
            .get_usage_stats(project.id, args.month.as_deref())
            .await?;
        output::format_usage_stats(&usage);
        return Ok(());
    }

    let stats = client.get_project_stats(project.id).await?;

    let usage = if args.usage || args.month.is_some() {
        Some(
            client
                .get_usage_stats(project.id, args.month.as_deref())
                .await?,
        )
    } else {
        None
    };

    if args.json {
        let val = serde_json::json!({
            "project": project.attributes.name,
            "stats": stats,
            "usage": usage,
        });
        println!("{}", serde_json::to_string_pretty(&val)?);
        return Ok(());
    }

    output::format_stats_dashboard(&project.attributes.name, &stats, usage.as_ref());
    Ok(())
}

fn is_valid_month(s: &str) -> bool {
    let parts: Vec<&str> = s.splitn(2, '-').collect();
    if parts.len() != 2 {
        return false;
    }
    let year_ok = parts[0].len() == 4 && parts[0].chars().all(|c| c.is_ascii_digit());
    let month_ok = !parts[1].is_empty()
        && parts[1].len() <= 2
        && parts[1].chars().all(|c| c.is_ascii_digit());
    year_ok && month_ok
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── month validation ────────────────────────────────────────────────────

    #[test]
    fn valid_month_yyyy_mm() {
        assert!(is_valid_month("2026-03"));
        assert!(is_valid_month("2024-12"));
        assert!(is_valid_month("2026-3"));
    }

    #[test]
    fn invalid_month_missing_dash() {
        assert!(!is_valid_month("202603"));
    }

    #[test]
    fn invalid_month_wrong_year_length() {
        assert!(!is_valid_month("26-03"));
        assert!(!is_valid_month("20260-03"));
    }

    #[test]
    fn invalid_month_non_numeric() {
        assert!(!is_valid_month("2026-ab"));
        assert!(!is_valid_month("abcd-03"));
    }

    #[test]
    fn invalid_month_empty() {
        assert!(!is_valid_month(""));
        assert!(!is_valid_month("-"));
        assert!(!is_valid_month("2026-"));
    }

    // ── StatsArgs defaults ──────────────────────────────────────────────────

    #[test]
    fn stats_args_defaults() {
        // Verify the struct can be constructed with defaults
        let args = StatsArgs {
            json: false,
            usage: false,
            month: None,
        };
        assert!(!args.json);
        assert!(!args.usage);
        assert!(args.month.is_none());
    }

    #[test]
    fn stats_args_with_flags() {
        let args = StatsArgs {
            json: true,
            usage: true,
            month: Some("2026-03".to_string()),
        };
        assert!(args.json);
        assert!(args.usage);
        assert_eq!(args.month.as_deref(), Some("2026-03"));
    }
}
