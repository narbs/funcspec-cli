use anyhow::{Context, Result};
use funcspec_client::FuncspecClient;
use std::sync::OnceLock;

use crate::config::{Config, LocalConfig};

static PROJECT_OVERRIDE: OnceLock<Option<String>> = OnceLock::new();
static TIMEOUT_OVERRIDE: OnceLock<Option<u64>> = OnceLock::new();

/// Default request timeout (seconds) when no --timeout override is given.
const DEFAULT_TIMEOUT_SECS: u64 = 30;
/// Default request timeout (seconds) for synchronous long-running audit calls.
pub const AUDIT_TIMEOUT_SECS: u64 = 180;

/// Set the global project override from --project flag (call once at startup).
pub fn set_project_override(project: Option<String>) {
    let _ = PROJECT_OVERRIDE.set(project);
}

/// Set the global request-timeout override from --timeout flag (call once at startup).
pub fn set_timeout_override(timeout_secs: Option<u64>) {
    let _ = TIMEOUT_OVERRIDE.set(timeout_secs);
}

/// Resolve the effective request timeout: explicit --timeout wins, else the
/// supplied fallback (e.g. AUDIT_TIMEOUT_SECS for the audit path), else the
/// client default.
fn effective_timeout(fallback_secs: u64) -> std::time::Duration {
    let secs = TIMEOUT_OVERRIDE
        .get()
        .and_then(|o| *o)
        .unwrap_or(fallback_secs);
    std::time::Duration::from_secs(secs)
}

fn project_override() -> Option<&'static str> {
    PROJECT_OVERRIDE.get().and_then(|o| o.as_deref())
}

/// Return the --project override slug if one was set, without resolving to a project ID.
/// Useful for commands that need the slug directly (e.g. `funcspec instructions`).
pub fn project_slug_override() -> Option<&'static str> {
    project_override()
}

/// Resolve the effective project slug using the full priority chain:
///
/// 1. `--project` flag (runtime override)
/// 2. `FUNCSPEC_PROJECT` env var
/// 3. Local `.funcspec` file (walks up from cwd)
/// 4. Global profile `default_project`
///
/// Returns `None` if nothing is configured.
pub fn resolve_project_slug<'a>(
    flag_override: Option<&'a str>,
    global_default: Option<&'a str>,
    local: Option<&'a LocalConfig>,
) -> Option<String> {
    // Static env var lookup (heap-allocated so we can return owned)
    let env_project = std::env::var("FUNCSPEC_PROJECT").ok();

    flag_override
        .map(str::to_owned)
        .or(env_project)
        .or_else(|| local.and_then(|lc| lc.project.clone()))
        .or_else(|| global_default.map(str::to_owned))
}

/// Build a client from the active profile. Returns (client, config).
pub fn client_and_config() -> Result<(FuncspecClient, Config)> {
    let config = Config::load()?;
    let profile = config
        .active_profile()
        .context("Not logged in. Run `funcspec auth login` to connect.")?;
    let client = FuncspecClient::with_timeout(
        &profile.host,
        &profile.api_key,
        effective_timeout(DEFAULT_TIMEOUT_SECS),
    )?;
    Ok((client, config))
}

/// Build a client and resolve the project ID. Uses `--project` override if provided,
/// otherwise falls back through the priority chain (env → local .funcspec → global default).
pub async fn client_and_project_with(
    project_override: Option<&str>,
) -> Result<(FuncspecClient, u64)> {
    let (client, config) = client_and_config()?;
    let profile = config.active_profile().unwrap();

    let cwd = std::env::current_dir().unwrap_or_default();
    let local = LocalConfig::find_and_load(&cwd);

    let project_slug = resolve_project_slug(
        project_override,
        profile.default_project.as_deref(),
        local.as_ref(),
    )
    .context(
        "No project specified. Use --project <slug>, set FUNCSPEC_PROJECT, \
         add a .funcspec file, or run `funcspec onboard`.",
    )?;

    let project = client
        .get_project(&project_slug)
        .await
        .with_context(|| format!("Project '{}' not found", project_slug))?;

    Ok((client, project.id))
}

/// Build a client and resolve the project ID. Checks --project override first.
pub async fn client_and_project() -> Result<(FuncspecClient, u64)> {
    client_and_project_with(project_override()).await
}

/// Like `client_and_project`, but the client uses the long audit timeout
/// (AUDIT_TIMEOUT_SECS) unless an explicit --timeout override is set. Use this
/// for synchronous long-running calls such as `ai audit`.
pub async fn client_and_project_audit() -> Result<(FuncspecClient, u64)> {
    let (_client, config) = client_and_config()?;
    let profile = config
        .active_profile()
        .context("Not logged in. Run `funcspec auth login` to connect.")?;

    let client = FuncspecClient::with_timeout(
        &profile.host,
        &profile.api_key,
        effective_timeout(AUDIT_TIMEOUT_SECS),
    )?;

    let cwd = std::env::current_dir().unwrap_or_default();
    let local = LocalConfig::find_and_load(&cwd);
    let project_slug = resolve_project_slug(
        project_override(),
        profile.default_project.as_deref(),
        local.as_ref(),
    )
    .context(
        "No project specified. Use --project <slug>, set FUNCSPEC_PROJECT, \
         add a .funcspec file, or run `funcspec onboard`.",
    )?;

    let project = client
        .get_project(&project_slug)
        .await
        .with_context(|| format!("Project '{}' not found", project_slug))?;

    Ok((client, project.id))
}
