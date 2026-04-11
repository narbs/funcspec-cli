use anyhow::{Context, Result, bail};
use clap::Args;

use crate::context::client_and_config;
use crate::output::OutputFormat;

/// Extract the `content` string from a raw agent_instructions API response.
/// Handles both `{ data: { attributes: { content } } }` and flatter shapes.
pub(crate) fn extract_content(raw: &serde_json::Value) -> Option<&str> {
    raw.get("data")
        .and_then(|d| {
            d.get("attributes")
                .and_then(|a| a.get("content"))
                .or_else(|| d.get("content"))
        })
        .or_else(|| raw.get("content"))
        .and_then(|c| c.as_str())
}

/// Arguments for `funcspec instructions`.
#[derive(Debug, Args)]
#[command(about = "Fetch live agent instructions for the current project")]
pub struct InstructionsArgs {
    /// Print the full JSON response instead of just the content field.
    /// Takes precedence over --format.
    #[arg(long)]
    pub raw: bool,
}

pub async fn run(args: InstructionsArgs, format: OutputFormat) -> Result<()> {
    let (client, config) = client_and_config()?;
    let profile = config
        .active_profile()
        .context("Not logged in. Run `funcspec auth login` to connect.")?;

    // Resolve project slug using the full priority chain
    let cwd = std::env::current_dir().unwrap_or_default();
    let local = crate::config::LocalConfig::find_and_load(&cwd);
    let slug = crate::context::resolve_project_slug(
        crate::context::project_slug_override(),
        profile.default_project.as_deref(),
        local.as_ref(),
    )
    .context(
        "No default project set. Run: funcspec config set project <slug>\n\
         Or run: funcspec onboard",
    )?;

    let raw_json = client
        .get_agent_instructions(&slug)
        .await
        .with_context(|| format!("Could not fetch instructions for project '{slug}'"))?;

    if args.raw {
        println!("{}", serde_json::to_string_pretty(&raw_json)?);
        return Ok(());
    }

    // Extract content field — API returns { data: { attributes: { content } } }
    // Also handle flatter shapes ({ data: { content } } or { content }) for robustness.
    let content = extract_content(&raw_json);

    match content {
        Some(text) => {
            if format == OutputFormat::Json {
                println!("{}", serde_json::json!({ "content": text }));
            } else {
                println!("{text}");
            }
        }
        None => {
            bail!(
                "No instructions content found for project '{slug}'.\n\
                 The project may not have agent instructions configured.\n\
                 Use --raw to inspect the full API response."
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── extract_content ────────────────────────────────────────────────────────

    #[test]
    fn extract_content_nested_attributes() {
        let raw = json!({
            "data": {
                "attributes": { "content": "# Hello" }
            }
        });
        assert_eq!(extract_content(&raw), Some("# Hello"));
    }

    #[test]
    fn extract_content_flat_data() {
        let raw = json!({ "data": { "content": "flat" } });
        assert_eq!(extract_content(&raw), Some("flat"));
    }

    #[test]
    fn extract_content_top_level() {
        let raw = json!({ "content": "top" });
        assert_eq!(extract_content(&raw), Some("top"));
    }

    #[test]
    fn extract_content_missing_returns_none() {
        let raw = json!({ "data": { "attributes": { "title": "no content here" } } });
        assert_eq!(extract_content(&raw), None);
    }

    #[test]
    fn extract_content_empty_object_returns_none() {
        assert_eq!(extract_content(&json!({})), None);
    }
}
