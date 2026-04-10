use anyhow::{Context, Result};
use funcspec_client::FuncspecClient;
use std::sync::OnceLock;

use crate::config::Config;

static PROJECT_OVERRIDE: OnceLock<Option<String>> = OnceLock::new();

/// Set the global project override from --project flag (call once at startup).
pub fn set_project_override(project: Option<String>) {
    let _ = PROJECT_OVERRIDE.set(project);
}

fn project_override() -> Option<&'static str> {
    PROJECT_OVERRIDE.get().and_then(|o| o.as_deref())
}

/// Return the --project override slug if one was set, without resolving to a project ID.
/// Useful for commands that need the slug directly (e.g. `funcspec instructions`).
pub fn project_slug_override() -> Option<&'static str> {
    project_override()
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

/// Build a client and resolve the project ID. Uses `--project` override if provided,
/// otherwise falls back to the default project from config.
pub async fn client_and_project_with(
    project_override: Option<&str>,
) -> Result<(FuncspecClient, u64)> {
    let (client, config) = client_and_config()?;
    let profile = config.active_profile().unwrap();

    let project_slug = project_override
        .or(profile.default_project.as_deref())
        .context(
            "No project specified. Use --project <slug> or run `funcspec projects set-default <slug>`.",
        )?;

    let project = client
        .get_project(project_slug)
        .await
        .with_context(|| format!("Project '{}' not found", project_slug))?;

    Ok((client, project.id))
}

/// Build a client and resolve the project ID. Checks --project override first.
pub async fn client_and_project() -> Result<(FuncspecClient, u64)> {
    client_and_project_with(project_override()).await
}
