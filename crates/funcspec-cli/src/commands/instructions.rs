use anyhow::{Context, Result, bail};
use clap::Args;

use crate::context::client_and_config;
use crate::output::OutputFormat;

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

    // Resolve project slug: global --project override takes priority, then default_project
    let slug = crate::context::project_slug_override()
        .or(profile.default_project.as_deref())
        .context(
            "No default project set. Run: funcspec config set project <slug>\n\
             Or run: funcspec onboard",
        )?
        .to_owned();

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
    let content = raw_json
        .get("data")
        .and_then(|d| d.get("attributes").and_then(|a| a.get("content")).or_else(|| d.get("content")))
        .or_else(|| raw_json.get("content"))
        .and_then(|c| c.as_str());

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
