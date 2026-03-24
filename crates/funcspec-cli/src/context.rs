use anyhow::{Context, Result};
use funcspec_client::FuncspecClient;

use crate::config::Config;

/// Output mode derived from --json / --quiet flags.
pub enum OutputMode {
    Table,
    Json,
    Quiet,
}

impl OutputMode {
    pub fn from_flags(json: bool, quiet: bool) -> Self {
        if json {
            OutputMode::Json
        } else if quiet {
            OutputMode::Quiet
        } else {
            OutputMode::Table
        }
    }
}

/// Build a client from the active profile. Returns (client, config).
pub fn client_and_config() -> Result<(FuncspecClient, Config)> {
    let config = Config::load()?;
    let profile = config
        .active_profile()
        .context("Not logged in. Run `funcspec auth login` to connect.")?;
    let client = FuncspecClient::new(&profile.host, &profile.api_key)?;
    Ok((client, config))
}

/// Build a client and resolve the project ID. Returns (client, project_id).
pub async fn client_and_project() -> Result<(FuncspecClient, u64)> {
    let (client, config) = client_and_config()?;
    let profile = config.active_profile().unwrap();

    let project_slug = profile.default_project.as_deref().context(
        "No default project set. Run `funcspec projects set-default <slug>` or pass --project.",
    )?;

    let project = client
        .get_project(project_slug)
        .await
        .with_context(|| format!("Project '{}' not found", project_slug))?;

    Ok((client, project.id))
}
