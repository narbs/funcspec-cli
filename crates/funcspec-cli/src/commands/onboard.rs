use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::Args;
use console::style;
use dialoguer::{Confirm, Input, Select};
use funcspec_client::FuncspecClient;

use crate::config::{Config, LocalConfig, Profile};

const LLM_CONFIG_FILES: &[&str] = &[
    "CLAUDE.md",
    "AGENTS.md",
    "OPENAI.md",
    "GEMINI.md",
    ".github/copilot-instructions.md",
    ".cursorrules",
    ".cursor/rules",
];

const LLM_BLURB: &str = "\n\n## FuncSpec Integration\n\nThis project uses [FuncSpec](https://funcspec.net) for spec management.\nRead `FUNCSPEC.md` in this directory for agent workflow instructions,\nCLI reference, and API endpoints.\n";

const FUNCSPEC_MD_TEMPLATE: &str = include_str!("../templates/funcspec_md.tmpl");

/// Arguments for `funcspec onboard`.
#[derive(Debug, Args)]
#[command(about = "Interactive setup wizard — authenticate, configure, and scaffold agent files")]
pub struct OnboardArgs {
    /// Run non-interactively (for CI). Requires FUNCSPEC_API_KEY or --api-key.
    #[arg(long)]
    pub non_interactive: bool,

    /// API key (overrides FUNCSPEC_API_KEY env var)
    #[arg(long, env = "FUNCSPEC_API_KEY")]
    pub api_key: Option<String>,

    /// Pre-select default project slug (skip interactive selection)
    #[arg(long)]
    pub set_project: Option<String>,

    /// Skip LLM agent config file integration step
    #[arg(long)]
    pub skip_llm: bool,

    /// Path to a specific LLM config file to update (repeatable)
    #[arg(long = "llm-config")]
    pub llm_configs: Vec<PathBuf>,

    /// Show what would happen without writing anything
    #[arg(long)]
    pub dry_run: bool,

    /// Target directory for FUNCSPEC.md and LLM config scanning
    #[arg(long, default_value = ".")]
    pub dir: PathBuf,
}

pub async fn run(args: OnboardArgs) -> Result<()> {
    let dir = args.dir.canonicalize().unwrap_or(args.dir.clone());

    eprintln!();
    eprintln!(
        "{}",
        style("Welcome to FuncSpec Onboarding").cyan().bold()
    );
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    eprintln!(
        "This wizard will help you authenticate, set a default project,\n\
         and scaffold agent instruction files in {}.",
        dir.display()
    );
    eprintln!();

    // ── Step 1: API key ───────────────────────────────────────────────────────
    eprintln!("{}", style("Step 1 — API Key").bold());

    let api_key = resolve_api_key(&args).await?;
    let host = "https://funcspec.net".to_string();

    let client =
        FuncspecClient::with_timeout(&host, &api_key, Duration::from_secs(10))
            .context("Failed to build API client")?;

    if args.dry_run {
        eprintln!("  {} API key validation (dry run)", style("[skip]").dim());
    } else {
        eprint!("  Validating API key… ");
        match client.validate_auth().await {
            Ok(_) => eprintln!("{}", style("✓").green().bold()),
            Err(e) => {
                eprintln!("{}", style("✗").red().bold());
                bail!("API key validation failed: {e}\nGet a key at: https://funcspec.net/settings#api-keys");
            }
        }
    }

    // Save API key to config
    if !args.dry_run {
        let mut config = Config::load()?;
        let profile_name = config.active_profile.clone();
        let existing = config.profiles.get(&profile_name).cloned();
        config.profiles.insert(
            profile_name,
            Profile {
                host: host.clone(),
                api_key: api_key.clone(),
                default_project: existing.and_then(|p| p.default_project),
            },
        );
        config.save()?;
    }

    // ── Step 2: Default project ───────────────────────────────────────────────
    eprintln!();
    eprintln!("{}", style("Step 2 — Default Project").bold());

    let (project_slug, org_slug, project_name, org_name) =
        resolve_project(&args, &client).await?;

    if !args.dry_run {
        // Write .funcspec in the target dir for per-directory project binding
        let local_path = dir.join(LocalConfig::FILE_NAME);
        let lc = LocalConfig { project: Some(project_slug.clone()) };
        lc.save_to_path(&local_path)
            .with_context(|| format!("Failed to write {}", local_path.display()))?;
        eprintln!("  {} .funcspec written (project = '{project_slug}')", style("✓").green().bold());

        // Also update global profile default so other directories work without a .funcspec
        let mut config = Config::load()?;
        let profile_name = config.active_profile.clone();
        if let Some(profile) = config.profiles.get_mut(&profile_name) {
            profile.default_project = Some(project_slug.clone());
        }
        config.save()?;
        eprintln!("  {} Global default project set to '{project_slug}'", style("✓").green().bold());
    } else {
        eprintln!("  {} Write .funcspec (project = '{project_slug}') (dry run)", style("[skip]").dim());
    }

    // ── Step 3: FUNCSPEC.md ───────────────────────────────────────────────────
    eprintln!();
    eprintln!("{}", style("Step 3 — FUNCSPEC.md").bold());

    let funcspec_md_path = dir.join("FUNCSPEC.md");
    let content = render_template(&org_name, &org_slug, &project_name, &project_slug);

    if args.dry_run {
        let action = if funcspec_md_path.exists() { "overwrite" } else { "create" };
        eprintln!("  [{}]    FUNCSPEC.md", action);
    } else {
        write_funcspec_md(&funcspec_md_path, &content, args.non_interactive)?;
    }

    // ── Step 4: LLM config files ──────────────────────────────────────────────
    if !args.skip_llm {
        eprintln!();
        eprintln!("{}", style("Step 4 — LLM Agent Config Files").bold());

        let llm_result = handle_llm_configs(&args, &dir)?;
        if !llm_result {
            eprintln!(
                "  {} No LLM config files updated.",
                style("⚠").yellow().bold()
            );
        }
    }

    // ── Step 5: Status summary ────────────────────────────────────────────────
    eprintln!();
    eprintln!("{}", style("Setup Complete").green().bold());
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    eprintln!("  {} API key", style("✓").green().bold());
    eprintln!("  {} .funcspec (project = '{project_slug}')", style("✓").green().bold());
    eprintln!("  {} Global default project: {project_slug}", style("✓").green().bold());
    eprintln!("  {} FUNCSPEC.md", style("✓").green().bold());
    eprintln!();
    eprintln!("Next steps:");
    eprintln!("  funcspec items list    — browse spec items");
    eprintln!("  funcspec stats         — project health overview");
    eprintln!("  funcspec doctor        — verify your environment");
    eprintln!();

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn resolve_api_key(args: &OnboardArgs) -> Result<String> {
    // --api-key flag (which also reads FUNCSPEC_API_KEY via clap env) takes priority
    if let Some(key) = &args.api_key {
        if !key.is_empty() {
            return Ok(key.clone());
        }
    }

    // Fall back to stored config
    if let Ok(config) = Config::load() {
        if let Some(profile) = config.active_profile() {
            if !profile.api_key.is_empty() {
                if args.non_interactive {
                    return Ok(profile.api_key);
                }
                let keep = Confirm::new()
                    .with_prompt(format!(
                        "An API key is already configured. Keep it?"
                    ))
                    .default(true)
                    .interact()?;
                if keep {
                    return Ok(profile.api_key);
                }
            }
        }
    }

    if args.non_interactive {
        bail!(
            "No API key found. Set FUNCSPEC_API_KEY or pass --api-key.\n\
             Get a key at: https://funcspec.net/settings#api-keys"
        );
    }

    eprintln!("  Get your API key at: https://funcspec.net/settings#api-keys");
    eprintln!("  (Opening in browser…)");
    let _ = open::that("https://funcspec.net/settings#api-keys");

    let key: String = Input::new()
        .with_prompt("  Paste your API key")
        .interact_text()?;

    if key.trim().is_empty() {
        bail!("No API key entered.");
    }

    Ok(key.trim().to_string())
}

async fn resolve_project(
    args: &OnboardArgs,
    client: &FuncspecClient,
) -> Result<(String, String, String, String)> {
    // Returns (project_slug, org_slug, project_name, org_name)
    // org_slug derived from project slug prefix (e.g. "tambit" from "tambit/my-proj") or fallback
    let projects = client
        .list_projects()
        .await
        .context("Failed to fetch projects. Check your API key and network connection.")?;

    if projects.is_empty() {
        bail!(
            "No projects found. Create one at: https://funcspec.net\n\
             Or accept an invite to an existing organisation."
        );
    }

    let chosen_slug = if let Some(slug) = &args.set_project {
        // Validate the provided slug exists
        if !projects.iter().any(|p| p.attributes.slug == *slug) {
            bail!(
                "Project '{slug}' not found or not accessible.\n\
                 Check the slug and your org membership, or re-run without --set-project to choose interactively."
            );
        }
        slug.clone()
    } else if args.non_interactive {
        // Default to first project
        projects[0].attributes.slug.clone()
    } else {
        let labels: Vec<String> = projects
            .iter()
            .map(|p| {
                if let Some(desc) = &p.attributes.description {
                    format!("{} — {}", p.attributes.slug, desc)
                } else {
                    p.attributes.slug.clone()
                }
            })
            .collect();

        let idx = Select::new()
            .with_prompt("  Select default project")
            .items(&labels)
            .default(0)
            .interact()?;

        projects[idx].attributes.slug.clone()
    };

    let project = projects
        .iter()
        .find(|p| p.attributes.slug == chosen_slug)
        .unwrap();

    // Derive org slug from the project slug if it contains '/' (e.g. "tambit/funcspec-cli"),
    // otherwise fetch one item and parse the org from its URL ("/tambit/funcspec-cli/F-1").
    let (org_slug, project_slug) = if chosen_slug.contains('/') {
        let parts: Vec<&str> = chosen_slug.splitn(2, '/').collect();
        (parts[0].to_string(), parts[1].to_string())
    } else {
        let org = resolve_org_slug(client, project.id).await;
        (org, chosen_slug.clone())
    };

    let org_name = titlecase(&org_slug.replace('-', " "));

    Ok((project_slug, org_slug, project.attributes.name.clone(), org_name))
}

fn render_template(org_name: &str, org_slug: &str, project_name: &str, project_slug: &str) -> String {
    FUNCSPEC_MD_TEMPLATE
        .replace("{{your_organization_title}}", org_name)
        .replace("{{your_organization_slug}}", org_slug)
        .replace("{{your_project_title}}", project_name)
        .replace("{{your_project_slug}}", project_slug)
}

fn write_funcspec_md(path: &Path, content: &str, non_interactive: bool) -> Result<()> {
    if path.exists() {
        let existing = std::fs::read_to_string(path)?;
        if existing == content {
            eprintln!("  {} FUNCSPEC.md is already up to date", style("✓").green().bold());
            return Ok(());
        }

        if !non_interactive {
            // Show a brief diff summary
            eprintln!("  FUNCSPEC.md already exists and differs from the generated template.");
            let overwrite = Confirm::new()
                .with_prompt("  Overwrite FUNCSPEC.md?")
                .default(false)
                .interact()?;

            if !overwrite {
                eprintln!("  {} FUNCSPEC.md skipped", style("–").dim());
                return Ok(());
            }
        }
    }

    // Atomic write: write to .tmp then rename
    let tmp = path.with_extension("md.tmp");
    std::fs::write(&tmp, content)
        .with_context(|| format!("Failed to write {}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    eprintln!("  {} FUNCSPEC.md written", style("✓").green().bold());
    Ok(())
}

/// Returns true if at least one LLM config was updated or already references FUNCSPEC.md.
pub fn handle_llm_configs(args: &OnboardArgs, dir: &Path) -> Result<bool> {
    // If specific paths provided, use those
    if !args.llm_configs.is_empty() {
        let mut any = false;
        for path in &args.llm_configs {
            any |= update_llm_config(path, args.dry_run, args.non_interactive)?;
        }
        return Ok(any);
    }

    // Scan known files
    let found: Vec<PathBuf> = LLM_CONFIG_FILES
        .iter()
        .map(|f| dir.join(f))
        .filter(|p| p.exists())
        .collect();

    if found.is_empty() {
        // Auto-create AGENTS.md as universal fallback
        let agents_path = dir.join("AGENTS.md");
        if args.dry_run {
            eprintln!("  [create]  AGENTS.md (FuncSpec Integration blurb)");
        } else {
            let content = format!("# AGENTS.md\n{LLM_BLURB}");
            let tmp = agents_path.with_extension("md.tmp");
            std::fs::write(&tmp, &content)?;
            std::fs::rename(&tmp, &agents_path)?;
            eprintln!("  {} AGENTS.md created", style("✓").green().bold());
        }
        return Ok(true);
    }

    let mut any_updated = false;
    for path in &found {
        any_updated |= update_llm_config(path, args.dry_run, args.non_interactive)?;
    }
    Ok(any_updated)
}

fn update_llm_config(path: &Path, dry_run: bool, non_interactive: bool) -> Result<bool> {
    let content = if path.exists() {
        std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?
    } else {
        String::new()
    };

    if content.contains("FUNCSPEC.md") {
        let name = path.display();
        eprintln!("  {} {name} already references FUNCSPEC.md", style("[skip]").dim());
        return Ok(true);
    }

    let name = path.display();

    if dry_run {
        let action = if path.exists() { "append" } else { "create" };
        eprintln!("  [{action}]   {name} (FuncSpec Integration blurb)");
        return Ok(true);
    }

    if !non_interactive {
        let prompt = if path.exists() {
            format!("  Append FuncSpec blurb to {name}?")
        } else {
            format!("  Create {name} with FuncSpec blurb?")
        };

        let confirmed = Confirm::new()
            .with_prompt(prompt)
            .default(true)
            .interact()?;

        if !confirmed {
            eprintln!("  {} {name} skipped", style("–").dim());
            return Ok(false);
        }
    }

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let new_content = if path.exists() {
        format!("{content}{LLM_BLURB}")
    } else {
        format!("# {}\n{LLM_BLURB}", path.file_name().unwrap_or_default().to_string_lossy())
    };

    let tmp = path.with_extension(
        path.extension()
            .map(|e| format!("{}.tmp", e.to_string_lossy()))
            .unwrap_or_else(|| "tmp".into()),
    );
    std::fs::write(&tmp, &new_content)?;
    std::fs::rename(&tmp, path)?;

    eprintln!("  {} {name} updated", style("✓").green().bold());
    Ok(true)
}

/// Return the org slug for the authenticated user via `GET /api/v1/settings`.
/// Falls back to `"your-org"` on any error.
async fn resolve_org_slug(client: &FuncspecClient, _project_id: u64) -> String {
    match client.get_org_slug().await {
        Ok(slug) if !slug.is_empty() => slug,
        _ => "your-org".to_string(),
    }
}

fn titlecase(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Check if the given directory appears fully onboarded.
pub fn is_onboarded(profile: &crate::config::Profile, dir: &Path) -> bool {
    let has_key = !profile.api_key.is_empty();
    let has_project = profile.default_project.is_some();

    let project_slug = profile.default_project.as_deref().unwrap_or("");
    let has_funcspec_md = check_funcspec_md_marker(dir, project_slug);

    let has_llm = LLM_CONFIG_FILES.iter().any(|f| {
        let p = dir.join(f);
        p.exists()
            && std::fs::read_to_string(&p)
                .map(|c| c.contains("FUNCSPEC.md"))
                .unwrap_or(false)
    });

    has_key && has_project && has_funcspec_md && has_llm
}

pub fn check_funcspec_md_marker(dir: &Path, project_slug: &str) -> bool {
    let path = dir.join("FUNCSPEC.md");
    if !path.exists() {
        return false;
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    content.to_lowercase().contains("funcspec")
        && (project_slug.is_empty() || content.contains(project_slug))
        && !project_slug.is_empty()
        && (content.contains(&format!("<!-- funcspec:v1:{project_slug} -->"))
            || content.contains(&format!("<!-- funcspec:v1:{project_slug}-->"))
            || content.contains(&format!("/{project_slug} -->"))
            || content.contains(&format!("/{project_slug}-->")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_dir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    // ── render_template ────────────────────────────────────────────────────────

    #[test]
    fn render_template_substitutes_all_placeholders() {
        let out = render_template("Acme Corp", "acme", "My Project", "my-project");
        assert!(out.contains("Acme Corp"));
        assert!(out.contains("acme"));
        assert!(out.contains("My Project"));
        assert!(out.contains("my-project"));
        assert!(!out.contains("{{your_organization_title}}"));
        assert!(!out.contains("{{your_organization_slug}}"));
        assert!(!out.contains("{{your_project_title}}"));
        assert!(!out.contains("{{your_project_slug}}"));
    }

    #[test]
    fn render_template_includes_footer_marker() {
        let out = render_template("Org", "org", "Proj", "proj");
        assert!(out.contains("<!-- funcspec:v1:org/proj -->"));
    }

    #[test]
    fn render_template_includes_project_url() {
        let out = render_template("Org", "org", "Proj", "proj");
        assert!(out.contains("https://funcspec.net/org/proj"));
    }

    // ── write_funcspec_md ──────────────────────────────────────────────────────

    #[test]
    fn write_funcspec_md_creates_new_file() {
        let dir = temp_dir();
        let path = dir.path().join("FUNCSPEC.md");
        write_funcspec_md(&path, "# Hello", true).unwrap();
        assert!(path.exists());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "# Hello");
    }

    #[test]
    fn write_funcspec_md_skips_when_identical() {
        let dir = temp_dir();
        let path = dir.path().join("FUNCSPEC.md");
        std::fs::write(&path, "same content").unwrap();
        // non_interactive=true so no prompt; content is identical so it should skip (not error)
        write_funcspec_md(&path, "same content", true).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "same content");
    }

    #[test]
    fn write_funcspec_md_overwrites_in_non_interactive() {
        let dir = temp_dir();
        let path = dir.path().join("FUNCSPEC.md");
        std::fs::write(&path, "old content").unwrap();
        write_funcspec_md(&path, "new content", true).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new content");
    }

    // ── check_funcspec_md_marker ───────────────────────────────────────────────

    #[test]
    fn marker_returns_false_when_no_file() {
        let dir = temp_dir();
        assert!(!check_funcspec_md_marker(dir.path(), "my-proj"));
    }

    #[test]
    fn marker_returns_true_for_valid_content() {
        let dir = temp_dir();
        let content = "# FuncSpec\nmy-proj stuff\n<!-- funcspec:v1:my-proj -->";
        std::fs::write(dir.path().join("FUNCSPEC.md"), content).unwrap();
        assert!(check_funcspec_md_marker(dir.path(), "my-proj"));
    }

    #[test]
    fn marker_returns_true_for_org_slug_form() {
        // Marker written as "org/slug" — slug-only check must still pass
        let dir = temp_dir();
        let content = "# FuncSpec\nmy-proj stuff\n<!-- funcspec:v1:tambit/my-proj -->";
        std::fs::write(dir.path().join("FUNCSPEC.md"), content).unwrap();
        assert!(check_funcspec_md_marker(dir.path(), "my-proj"));
    }

    #[test]
    fn marker_returns_false_when_slug_missing() {
        let dir = temp_dir();
        let content = "# FuncSpec\nother-proj\n<!-- funcspec:v1:other-proj -->";
        std::fs::write(dir.path().join("FUNCSPEC.md"), content).unwrap();
        assert!(!check_funcspec_md_marker(dir.path(), "my-proj"));
    }

    #[test]
    fn marker_returns_false_when_marker_absent() {
        let dir = temp_dir();
        let content = "# FuncSpec\nmy-proj stuff\n(no marker here)";
        std::fs::write(dir.path().join("FUNCSPEC.md"), content).unwrap();
        assert!(!check_funcspec_md_marker(dir.path(), "my-proj"));
    }

    #[test]
    fn marker_returns_false_with_empty_slug() {
        // Empty slug: marker check requires `<!-- funcspec:v1: -->` literally — without a real
        // slug the marker is never present, so the check returns false.
        let dir = temp_dir();
        std::fs::write(dir.path().join("FUNCSPEC.md"), "# FuncSpec intro").unwrap();
        assert!(!check_funcspec_md_marker(dir.path(), ""));
    }

    // ── update_llm_config ─────────────────────────────────────────────────────

    #[test]
    fn update_llm_config_creates_new_file_non_interactive() {
        let dir = temp_dir();
        let path = dir.path().join("CLAUDE.md");
        update_llm_config(&path, false, true).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("FUNCSPEC.md"));
        assert!(content.contains("FuncSpec Integration"));
    }

    #[test]
    fn update_llm_config_appends_to_existing_non_interactive() {
        let dir = temp_dir();
        let path = dir.path().join("CLAUDE.md");
        std::fs::write(&path, "# Existing content\n").unwrap();
        update_llm_config(&path, false, true).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Existing content"));
        assert!(content.contains("FUNCSPEC.md"));
    }

    #[test]
    fn update_llm_config_skips_when_already_referenced() {
        let dir = temp_dir();
        let path = dir.path().join("CLAUDE.md");
        std::fs::write(&path, "Read `FUNCSPEC.md` for instructions.\n").unwrap();
        let result = update_llm_config(&path, false, true).unwrap();
        assert!(result);
        // Content unchanged
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "Read `FUNCSPEC.md` for instructions.\n"
        );
    }

    #[test]
    fn update_llm_config_dry_run_does_not_write() {
        let dir = temp_dir();
        let path = dir.path().join("CLAUDE.md");
        update_llm_config(&path, true, true).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn update_llm_config_creates_parent_dirs() {
        let dir = temp_dir();
        let path = dir.path().join(".github").join("copilot-instructions.md");
        update_llm_config(&path, false, true).unwrap();
        assert!(path.exists());
    }

    // ── is_onboarded ──────────────────────────────────────────────────────────

    #[test]
    fn is_onboarded_false_when_no_key() {
        let dir = temp_dir();
        let profile = crate::config::Profile {
            host: "https://funcspec.net".into(),
            api_key: "".into(),
            default_project: Some("my-proj".into()),
        };
        assert!(!is_onboarded(&profile, dir.path()));
    }

    #[test]
    fn is_onboarded_false_when_no_project() {
        let dir = temp_dir();
        let profile = crate::config::Profile {
            host: "https://funcspec.net".into(),
            api_key: "mykey".into(),
            default_project: None,
        };
        assert!(!is_onboarded(&profile, dir.path()));
    }

    #[test]
    fn is_onboarded_true_when_all_criteria_met() {
        let dir = temp_dir();
        // Write valid FUNCSPEC.md
        let funcspec_content = "# FuncSpec\nmy-proj\n<!-- funcspec:v1:my-proj -->";
        std::fs::write(dir.path().join("FUNCSPEC.md"), funcspec_content).unwrap();
        // Write LLM config that references FUNCSPEC.md
        std::fs::write(dir.path().join("CLAUDE.md"), "Read `FUNCSPEC.md`").unwrap();

        let profile = crate::config::Profile {
            host: "https://funcspec.net".into(),
            api_key: "mykey".into(),
            default_project: Some("my-proj".into()),
        };
        assert!(is_onboarded(&profile, dir.path()));
    }
}
