use anyhow::{Result, bail};
use console::style;
use rust_i18n::t;

use crate::config::{Config, LocalConfig};

pub enum ConfigCmd {
    Set {
        key: String,
        value: String,
        local: bool,
    },
    Get {
        key: String,
    },
    List,
    SetProfile {
        name: String,
    },
}

pub fn build_command() -> clap::Command {
    clap::Command::new("config")
        .about(t!("cmd.config.about").to_string())
        .arg_required_else_help(true)
        .subcommand(
            clap::Command::new("set")
                .about(t!("cmd.config.set.about").to_string())
                .arg(
                    clap::Arg::new("key")
                        .required(true)
                        .help(t!("cmd.config.set.key").to_string()),
                )
                .arg(
                    clap::Arg::new("value")
                        .required(true)
                        .help(t!("cmd.config.set.value").to_string()),
                )
                .arg(
                    clap::Arg::new("local")
                        .long("local")
                        .action(clap::ArgAction::SetTrue)
                        .help(t!("cmd.config.set.local").to_string()),
                ),
        )
        .subcommand(
            clap::Command::new("get")
                .about(t!("cmd.config.get.about").to_string())
                .arg(
                    clap::Arg::new("key")
                        .required(true)
                        .help(t!("cmd.config.get.key").to_string()),
                ),
        )
        .subcommand(clap::Command::new("list").about(t!("cmd.config.list.about").to_string()))
        .subcommand(
            clap::Command::new("set-profile")
                .about(t!("cmd.config.set_profile.about").to_string())
                .arg(
                    clap::Arg::new("name")
                        .required(true)
                        .help(t!("cmd.config.set_profile.name").to_string()),
                ),
        )
}

pub async fn dispatch(matches: &clap::ArgMatches) -> Result<()> {
    let cmd = match matches.subcommand() {
        Some(("set", m)) => ConfigCmd::Set {
            key: m.get_one::<String>("key").unwrap().clone(),
            value: m.get_one::<String>("value").unwrap().clone(),
            local: m.get_flag("local"),
        },
        Some(("get", m)) => ConfigCmd::Get {
            key: m.get_one::<String>("key").unwrap().clone(),
        },
        Some(("list", _)) => ConfigCmd::List,
        Some(("set-profile", m)) => ConfigCmd::SetProfile {
            name: m.get_one::<String>("name").unwrap().clone(),
        },
        _ => {
            build_command().print_help().ok();
            return Ok(());
        }
    };
    run(cmd).await
}

pub async fn run(cmd: ConfigCmd) -> Result<()> {
    match cmd {
        ConfigCmd::Set { key, value, local } => {
            // --local only makes sense for "project"
            if local {
                match key.as_str() {
                    "project" | "default_project" => {
                        let path = std::env::current_dir()
                            .unwrap_or_default()
                            .join(LocalConfig::FILE_NAME);
                        let mut lc = if path.exists() {
                            LocalConfig::load_from_path(&path)?
                        } else {
                            LocalConfig::default()
                        };
                        lc.project = Some(value.clone());
                        lc.save_to_path(&path)?;
                        eprintln!(
                            "Set {} = {} (local: {})",
                            style(&key).cyan(),
                            style(&value).green(),
                            style(path.display().to_string()).dim(),
                        );
                        return Ok(());
                    }
                    k => bail!("--local only applies to 'project'. Got: '{k}'"),
                }
            }

            let mut config = Config::load()?;
            match key.as_str() {
                "project" | "default_project" => {
                    let profile_name = config.active_profile.clone();
                    let profile = config.profiles.get_mut(&profile_name).ok_or_else(|| {
                        anyhow::anyhow!("No active profile. Run `funcspec auth login` first.")
                    })?;
                    profile.default_project = Some(value.clone());
                }
                "host" => {
                    let profile_name = config.active_profile.clone();
                    let profile = config.profiles.get_mut(&profile_name).ok_or_else(|| {
                        anyhow::anyhow!("No active profile. Run `funcspec auth login` first.")
                    })?;
                    profile.host = value.clone();
                }
                "api_key" | "key" => {
                    let profile_name = config.active_profile.clone();
                    let profile = config.profiles.get_mut(&profile_name).ok_or_else(|| {
                        anyhow::anyhow!("No active profile. Run `funcspec auth login` first.")
                    })?;
                    profile.api_key = value.clone();
                }
                "profile" => {
                    if !config.profiles.contains_key(&value) {
                        bail!(
                            "Profile '{}' does not exist. Available: {}",
                            value,
                            config
                                .profiles
                                .keys()
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                    config.active_profile = value.clone();
                }
                k => {
                    bail!("Unknown config key: '{k}'. Valid keys: project, host, api_key, profile")
                }
            }
            config.save()?;
            eprintln!("Set {} = {}", style(&key).cyan(), style(&value).green());
            Ok(())
        }

        ConfigCmd::Get { key } => {
            let config = Config::load()?;
            match key.as_str() {
                "project" | "default_project" => {
                    match config
                        .active_profile()
                        .and_then(|p| p.default_project.clone())
                    {
                        Some(v) => println!("{v}"),
                        None => eprintln!("(not set)"),
                    }
                }
                "host" => match config.active_profile().map(|p| p.host.clone()) {
                    Some(v) => println!("{v}"),
                    None => eprintln!("(not set)"),
                },
                "profile" => println!("{}", config.active_profile),
                "api_key" | "key" => match config.active_profile().map(|p| p.api_key.clone()) {
                    Some(v) => println!("{v}"),
                    None => eprintln!("(not set)"),
                },
                k => bail!("Unknown config key: '{k}'"),
            }
            Ok(())
        }

        ConfigCmd::List => {
            let config = Config::load()?;

            // Show local .funcspec if present
            let cwd = std::env::current_dir().unwrap_or_default();
            #[allow(clippy::collapsible_if)]
            if let Some(local_path) = LocalConfig::find(&cwd) {
                if let Ok(lc) = LocalConfig::load_from_path(&local_path) {
                    eprintln!(
                        "Local config: {}",
                        style(local_path.display().to_string()).dim()
                    );
                    if let Some(ref proj) = lc.project {
                        eprintln!(
                            "  project: {} {}",
                            style(proj).green(),
                            style("(local override)").dim()
                        );
                    }
                    eprintln!();
                }
            }

            eprintln!(
                "Active profile: {}",
                style(&config.active_profile).cyan().bold()
            );
            eprintln!();
            for (name, profile) in &config.profiles {
                let active = if name == &config.active_profile {
                    " (active)"
                } else {
                    ""
                };
                eprintln!("{}{}:", style(name).cyan().bold(), style(active).dim());
                eprintln!("  host:    {}", profile.host);
                let masked = if profile.api_key.len() > 8 {
                    format!(
                        "{}…{}",
                        &profile.api_key[..4],
                        &profile.api_key[profile.api_key.len() - 4..]
                    )
                } else {
                    "*".repeat(profile.api_key.len())
                };
                eprintln!("  api_key: {masked}");
                if let Some(ref proj) = profile.default_project {
                    eprintln!("  project: {proj}");
                }
            }
            Ok(())
        }

        ConfigCmd::SetProfile { name } => {
            let mut config = Config::load()?;
            if !config.profiles.contains_key(&name) {
                bail!(
                    "Profile '{}' does not exist. Available: {}",
                    name,
                    config
                        .profiles
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            config.active_profile = name.clone();
            config.save()?;
            eprintln!("Switched to profile {}", style(&name).cyan());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_command_set_parses_key_value() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["config", "set", "project", "my-proj"])
            .unwrap();
        let sub = m.subcommand_matches("set").unwrap();
        assert_eq!(sub.get_one::<String>("key").unwrap(), "project");
        assert_eq!(sub.get_one::<String>("value").unwrap(), "my-proj");
        assert!(!sub.get_flag("local"));
    }

    #[test]
    fn build_command_set_local_flag() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["config", "set", "project", "my-proj", "--local"])
            .unwrap();
        let sub = m.subcommand_matches("set").unwrap();
        assert!(sub.get_flag("local"));
    }

    #[test]
    fn build_command_get_parses_key() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["config", "get", "project"])
            .unwrap();
        let sub = m.subcommand_matches("get").unwrap();
        assert_eq!(sub.get_one::<String>("key").unwrap(), "project");
    }

    #[test]
    fn build_command_set_profile_parses_name() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["config", "set-profile", "work"])
            .unwrap();
        let sub = m.subcommand_matches("set-profile").unwrap();
        assert_eq!(sub.get_one::<String>("name").unwrap(), "work");
    }
}
