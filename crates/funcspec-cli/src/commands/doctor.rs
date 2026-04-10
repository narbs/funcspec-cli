use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::Args;
use console::style;
use serde::{Deserialize, Serialize};

use crate::config::Config;

const LLM_CONFIG_FILES: &[&str] = &[
    "CLAUDE.md",
    "AGENTS.md",
    "OPENAI.md",
    "GEMINI.md",
    ".github/copilot-instructions.md",
    ".cursorrules",
    ".cursor/rules",
];

const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/funcspec/funcspec-cli/releases/latest";

const VERSION_CACHE_FILE: &str = "version_cache.json";

/// Arguments for `funcspec doctor`.
#[derive(Debug, Args)]
#[command(about = "Check the local environment and show a health report")]
pub struct DoctorArgs {
    /// Output results as JSON
    #[arg(long)]
    pub json: bool,

    /// Automatically fix issues where possible
    #[arg(long)]
    pub fix: bool,

    /// Auto-confirm fixes without prompting (use with --fix)
    #[arg(long)]
    pub yes: bool,

    /// Only show failures and warnings
    #[arg(long)]
    pub quiet: bool,

    /// Disable ANSI color output (also honoured via NO_COLOR env var)
    #[arg(long)]
    pub no_color: bool,

    /// Show additional detail for each check
    #[arg(long)]
    pub verbose: bool,

    /// Timeout in seconds for each network-dependent check
    #[arg(long, default_value = "10")]
    pub timeout: u64,

    /// Target directory for FUNCSPEC.md and LLM config scanning
    #[arg(long, default_value = ".")]
    pub dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Fail,
    Warn,
    Skipped,
}

#[derive(Debug, Serialize)]
pub struct CheckResult {
    pub name: &'static str,
    pub status: CheckStatus,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl CheckResult {
    fn pass(name: &'static str, detail: impl Into<String>) -> Self {
        Self { name, status: CheckStatus::Pass, detail: detail.into(), fix: None, reason: None }
    }
    fn fail(name: &'static str, detail: impl Into<String>, fix: Option<String>) -> Self {
        Self { name, status: CheckStatus::Fail, detail: detail.into(), fix, reason: None }
    }
    fn warn(name: &'static str, detail: impl Into<String>) -> Self {
        Self { name, status: CheckStatus::Warn, detail: detail.into(), fix: None, reason: None }
    }
    fn skipped(name: &'static str, reason: impl Into<String>) -> Self {
        Self { name, status: CheckStatus::Skipped, detail: String::new(), fix: None, reason: Some(reason.into()) }
    }
}

#[derive(Debug, Serialize)]
struct DoctorOutput {
    version: &'static str,
    checks_passed: usize,
    checks_total: usize,
    checks: Vec<CheckResult>,
}

pub async fn run(args: DoctorArgs) -> Result<()> {
    let no_color = args.no_color || std::env::var("NO_COLOR").is_ok();
    let dir = args.dir.canonicalize().unwrap_or(args.dir.clone());

    if !args.json {
        eprintln!();
        eprintln!("funcspec doctor");
        eprintln!("═══════════════");
        eprintln!();
    }

    let mut results: Vec<CheckResult> = Vec::new();

    // Check 1: CLI version (always runs)
    results.push(check_cli_version(args.timeout, args.verbose).await);

    // Check 2: API key configured (always runs)
    let key_check = check_api_key_configured();
    let key_ok = key_check.status == CheckStatus::Pass;
    results.push(key_check);

    // Check 3: API key valid (skipped if check 2 failed)
    let api_ok = if key_ok {
        let c = check_api_key_valid(args.timeout).await;
        let ok = c.status == CheckStatus::Pass;
        results.push(c);
        ok
    } else {
        results.push(CheckResult::skipped("api_key_valid", "no API key configured"));
        false
    };

    // Check 4: Default project set (always runs)
    let proj_check = check_default_project_set();
    let proj_ok = proj_check.status == CheckStatus::Pass;
    let project_slug = extract_project_slug();
    results.push(proj_check);

    // Check 5: Project accessible (skipped if check 3 or 4 failed)
    if api_ok && proj_ok {
        results.push(check_project_accessible(args.timeout, &project_slug).await);
    } else {
        let reason = if !api_ok { "API key invalid" } else { "no default project set" };
        results.push(CheckResult::skipped("project_access", reason));
    }

    // Checks 6 & 7: Always run (local filesystem)
    results.push(check_funcspec_md(&dir, &project_slug));
    results.push(check_llm_config(&dir));

    // ── Output ────────────────────────────────────────────────────────────────
    let passes = results.iter().filter(|r| r.status == CheckStatus::Pass).count();
    let total = results.len();

    if args.json {
        let output = DoctorOutput {
            version: env!("CARGO_PKG_VERSION"),
            checks_passed: passes,
            checks_total: total,
            checks: results,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    for result in &results {
        print_check(result, no_color, args.quiet, args.verbose);
    }

    eprintln!();
    if passes == total {
        eprintln!("{}", style(format!("{passes}/{total} checks passed")).green().bold());
    } else {
        eprintln!("{}/{} checks passed", passes, total);
    }
    eprintln!();

    // Exit with appropriate code
    let has_failures = results.iter().any(|r| r.status == CheckStatus::Fail);
    if has_failures {
        std::process::exit(1);
    }

    Ok(())
}

// ── Individual checks ─────────────────────────────────────────────────────────

async fn check_cli_version(timeout_secs: u64, verbose: bool) -> CheckResult {
    let current = env!("CARGO_PKG_VERSION");

    // Try cache first
    if let Some(cached) = load_version_cache(current) {
        if verbose {
            eprintln!("  [version cache hit]");
        }
        return version_result(current, &cached);
    }

    // Fetch from GitHub
    let client = reqwest::Client::new();
    let result = client
        .get(GITHUB_RELEASES_URL)
        .header(
            "User-Agent",
            format!("funcspec-cli/{}", env!("CARGO_PKG_VERSION")),
        )
        .timeout(Duration::from_secs(timeout_secs))
        .send()
        .await;

    match result {
        Ok(resp) if resp.status().is_success() => {
            let json: serde_json::Value = match resp.json().await {
                Ok(j) => j,
                Err(_) => {
                    return CheckResult::warn(
                        "cli_version",
                        format!("v{current} (could not parse GitHub response)"),
                    )
                }
            };
            let tag = json
                .get("tag_name")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .trim_start_matches('v')
                .to_string();

            save_version_cache(current, &tag);
            version_result(current, &tag)
        }
        _ => CheckResult::warn(
            "cli_version",
            format!("v{current} (could not reach GitHub, version check skipped)"),
        ),
    }
}

fn version_result(current: &str, latest: &str) -> CheckResult {
    if latest.is_empty() || latest == current {
        CheckResult::pass("cli_version", format!("v{current} (latest)"))
    } else {
        CheckResult::fail(
            "cli_version",
            format!("v{current} (latest: v{latest})"),
            Some("curl -fsSL https://funcspec.net/install.sh | bash".into()),
        )
    }
}

#[derive(Serialize, Deserialize)]
struct VersionCache {
    latest: String,
    checked_at: DateTime<Utc>,
    cli_version: String,
}

fn version_cache_path() -> Option<std::path::PathBuf> {
    Config::config_path().ok()?.parent().map(|p| p.join(VERSION_CACHE_FILE))
}

fn load_version_cache(current_version: &str) -> Option<String> {
    let path = version_cache_path()?;
    let data = std::fs::read_to_string(&path).ok()?;
    let cache: VersionCache = serde_json::from_str(&data).ok()?;

    // Cache is valid if: within 24h AND was written by same CLI version
    let age = Utc::now() - cache.checked_at;
    if cache.cli_version != current_version || age.num_hours() >= 24 {
        return None;
    }
    Some(cache.latest)
}

fn save_version_cache(current_version: &str, latest: &str) {
    let Some(path) = version_cache_path() else { return };
    let cache = VersionCache {
        latest: latest.to_string(),
        checked_at: Utc::now(),
        cli_version: current_version.to_string(),
    };
    if let Ok(data) = serde_json::to_string(&cache) {
        let _ = std::fs::write(&path, data);
    }
}

fn check_api_key_configured() -> CheckResult {
    let env_key = std::env::var("FUNCSPEC_API_KEY")
        .ok()
        .filter(|k| !k.is_empty());

    if env_key.is_some() {
        return CheckResult::pass("api_key_set", "configured (FUNCSPEC_API_KEY env var)");
    }

    let config = Config::load().unwrap_or_default();
    let has_key = config
        .active_profile()
        .map(|p| !p.api_key.is_empty())
        .unwrap_or(false);

    if has_key {
        CheckResult::pass("api_key_set", "configured")
    } else {
        CheckResult::fail(
            "api_key_set",
            "no API key found",
            Some("funcspec onboard".into()),
        )
    }
}

async fn check_api_key_valid(timeout_secs: u64) -> CheckResult {
    let config = Config::load().unwrap_or_default();
    let profile = match config.active_profile() {
        Some(p) => p,
        None => return CheckResult::fail("api_key_valid", "no active profile", None),
    };

    let Ok(client_with_timeout) = funcspec_client::FuncspecClient::with_timeout(
        &profile.host,
        &profile.api_key,
        Duration::from_secs(timeout_secs),
    ) else {
        return CheckResult::fail("api_key_valid", "could not build client", None);
    };

    match client_with_timeout.validate_auth().await {
        Ok(user) => CheckResult::pass("api_key_valid", format!("authenticated ({})", user.name)),
        Err(e) if matches!(e, funcspec_client::Error::Auth(_)) => CheckResult::fail(
            "api_key_valid",
            format!("invalid API key: {e}"),
            Some("https://funcspec.net/settings#api-keys".into()),
        ),
        Err(e) => CheckResult::fail(
            "api_key_valid",
            format!("network error: {e}. Check connectivity to funcspec.net"),
            None,
        ),
    }
}

fn check_default_project_set() -> CheckResult {
    let config = Config::load().unwrap_or_default();
    let slug = config
        .active_profile()
        .and_then(|p| p.default_project);

    match slug {
        Some(s) if !s.is_empty() => CheckResult::pass("default_project", s),
        _ => CheckResult::fail(
            "default_project",
            "no default project configured",
            Some("funcspec config set project <slug>  OR  funcspec onboard".into()),
        ),
    }
}

fn extract_project_slug() -> String {
    Config::load()
        .ok()
        .and_then(|c| c.active_profile())
        .and_then(|p| p.default_project)
        .unwrap_or_default()
}

async fn check_project_accessible(timeout_secs: u64, slug: &str) -> CheckResult {
    if slug.is_empty() {
        return CheckResult::skipped("project_access", "no project slug");
    }

    let config = Config::load().unwrap_or_default();
    let profile = match config.active_profile() {
        Some(p) => p,
        None => return CheckResult::fail("project_access", "no active profile", None),
    };

    let Ok(client) = funcspec_client::FuncspecClient::with_timeout(
        &profile.host,
        &profile.api_key,
        Duration::from_secs(timeout_secs),
    ) else {
        return CheckResult::fail("project_access", "could not build client", None);
    };

    match client.get_project(slug).await {
        Ok(p) => CheckResult::pass("project_access", p.attributes.name),
        Err(e) if matches!(e, funcspec_client::Error::NotFound(_)) => CheckResult::fail(
            "project_access",
            format!("project '{slug}' not found"),
            Some("Check org membership and project slug".into()),
        ),
        Err(e) if matches!(e, funcspec_client::Error::Forbidden(_)) => CheckResult::fail(
            "project_access",
            format!("access denied to project '{slug}'"),
            Some("Check org membership".into()),
        ),
        Err(e) => CheckResult::fail("project_access", format!("error: {e}"), None),
    }
}

fn check_funcspec_md(dir: &Path, project_slug: &str) -> CheckResult {
    let path = dir.join("FUNCSPEC.md");

    if !path.exists() {
        return CheckResult::fail(
            "funcspec_md",
            format!("FUNCSPEC.md not found in {}", dir.display()),
            Some("funcspec onboard".into()),
        );
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            return CheckResult::fail("funcspec_md", format!("could not read FUNCSPEC.md: {e}"), None)
        }
    };

    let has_word = content.to_lowercase().contains("funcspec");
    let has_slug = project_slug.is_empty() || content.contains(project_slug);
    let marker = format!("<!-- funcspec:v1:{project_slug}");
    let has_marker = !project_slug.is_empty() && content.contains(&marker);

    if has_word && has_slug && has_marker {
        CheckResult::pass("funcspec_md", "present and current")
    } else {
        let mut issues = Vec::new();
        if !has_word { issues.push("missing 'funcspec' reference"); }
        if !has_slug && !project_slug.is_empty() { issues.push("project slug not found"); }
        if !has_marker && !project_slug.is_empty() { issues.push("version marker missing or mismatched"); }
        CheckResult::warn(
            "funcspec_md",
            format!("present but may be stale ({})", issues.join(", ")),
        )
    }
}

fn check_llm_config(dir: &Path) -> CheckResult {
    let found: Vec<PathBuf> = LLM_CONFIG_FILES
        .iter()
        .map(|f| dir.join(f))
        .filter(|p| p.exists())
        .collect();

    if found.is_empty() {
        return CheckResult::fail(
            "llm_config",
            "no LLM agent config files found",
            Some("funcspec onboard".into()),
        );
    }

    let referencing: Vec<String> = found
        .iter()
        .filter(|p| {
            std::fs::read_to_string(p)
                .map(|c| c.contains("FUNCSPEC.md"))
                .unwrap_or(false)
        })
        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().into_owned())
        .collect();

    let not_referencing: Vec<String> = found
        .iter()
        .filter(|p| {
            !std::fs::read_to_string(p)
                .map(|c| c.contains("FUNCSPEC.md"))
                .unwrap_or(false)
        })
        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().into_owned())
        .collect();

    if referencing.is_empty() {
        CheckResult::fail(
            "llm_config",
            format!(
                "{} found but none reference FUNCSPEC.md",
                found.len()
            ),
            Some("funcspec onboard".into()),
        )
    } else {
        let detail = if not_referencing.is_empty() {
            format!("{} references FUNCSPEC.md", referencing.join(", "))
        } else {
            format!(
                "{} references FUNCSPEC.md ({} do not)",
                referencing.join(", "),
                not_referencing.join(", ")
            )
        };
        CheckResult::pass("llm_config", detail)
    }
}

// ── Display ───────────────────────────────────────────────────────────────────

fn print_check(result: &CheckResult, no_color: bool, quiet: bool, _verbose: bool) {
    let is_pass = result.status == CheckStatus::Pass;

    if quiet && is_pass {
        return;
    }

    let symbol = match result.status {
        CheckStatus::Pass => {
            if no_color { "✓".to_string() } else { style("✓").green().bold().to_string() }
        }
        CheckStatus::Fail => {
            if no_color { "✗".to_string() } else { style("✗").red().bold().to_string() }
        }
        CheckStatus::Warn => {
            if no_color { "⚠".to_string() } else { style("⚠").yellow().bold().to_string() }
        }
        CheckStatus::Skipped => {
            if no_color { "-".to_string() } else { style("-").dim().to_string() }
        }
    };

    let label = format!("{:<20}", result.name.replace('_', " "));
    let detail = if result.status == CheckStatus::Skipped {
        format!("skipped — {}", result.reason.as_deref().unwrap_or(""))
    } else {
        result.detail.clone()
    };

    eprintln!("  {symbol} {label}  {detail}");

    if let Some(fix) = &result.fix {
        eprintln!("                        → {fix}");
    }
}
