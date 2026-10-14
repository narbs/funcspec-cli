use anyhow::Result;
use rust_i18n::t;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn build_command() -> clap::Command {
    clap::Command::new("version")
        .about(t!("cmd.version.about").to_string())
        .subcommand(
            clap::Command::new("assign")
                .about("Add a spec item to a release version (by semver e.g. 0.2.0, or id)")
                .arg(
                    clap::Arg::new("release_version")
                        .required(true)
                        .help("Target release version: semver (0.2.0) or id"),
                )
                .arg(
                    clap::Arg::new("item")
                        .long("item")
                        .num_args(1..)
                        .value_delimiter(',')
                        .required(true)
                        .help("Spec item permalink(s) or id(s) to add (comma-separated, e.g. T-414)"),
                ),
        )
        .subcommand(
            clap::Command::new("unassign")
                .about("Remove a spec item from a release version (by semver or id)")
                .arg(
                    clap::Arg::new("release_version")
                        .required(true)
                        .help("Release version: semver (0.2.0) or id"),
                )
                .arg(
                    clap::Arg::new("item")
                        .long("item")
                        .required(true)
                        .help("Spec item permalink or id to remove (e.g. T-414)"),
                ),
        )
}

pub async fn dispatch(
    matches: &clap::ArgMatches,
    _format: crate::output::OutputFormat,
) -> Result<()> {
    match matches.subcommand() {
        Some(("assign", m)) => {
            let version = m.get_one::<String>("release_version").unwrap().clone();
            let items: Vec<String> = m
                .get_many::<String>("item")
                .map(|v| v.cloned().collect())
                .unwrap_or_default();
            let (client, project_id) = crate::context::client_and_project().await?;
            client
                .add_items_to_version(project_id, &version, &items)
                .await?;
            println!("Added {} to release version {}", items.join(", "), version);
            Ok(())
        }
        Some(("unassign", m)) => {
            let version = m.get_one::<String>("release_version").unwrap().clone();
            let item = m.get_one::<String>("item").unwrap().clone();
            let (client, project_id) = crate::context::client_and_project().await?;
            client
                .remove_item_from_version(project_id, &version, &item)
                .await?;
            println!("Removed {} from release version {}", item, version);
            Ok(())
        }
        _ => run(),
    }
}
const PKG_NAME: &str = env!("CARGO_PKG_NAME");

pub fn run() -> Result<()> {
    println!("{PKG_NAME} {VERSION}");
    println!("Build: Rust {}", rustc_version());
    if let Some(host) = configured_host() {
        println!("Host:  {host}");
    } else {
        println!("Host:  (not configured — run `funcspec auth login`)");
    }
    Ok(())
}

fn configured_host() -> Option<String> {
    // Check env var first
    if let Ok(h) = std::env::var("FUNCSPEC_HOST") {
        return Some(h);
    }
    // Try loading from config
    crate::config::Config::load()
        .ok()
        .and_then(|c| c.active_profile())
        .map(|p| p.host)
}

fn rustc_version() -> String {
    // Captured at build time via RUSTC_VERSION env var, if set
    option_env!("RUSTC_VERSION")
        .unwrap_or("unknown")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_string_is_semver() {
        // Just verify it's set and non-empty
        assert!(!VERSION.is_empty());
        // Should look like major.minor.patch
        let parts: Vec<&str> = VERSION.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn version_run_succeeds() {
        // Should not panic
        run().unwrap();
    }
}
