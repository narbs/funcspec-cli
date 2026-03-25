use anyhow::{Result, bail};
use clap::Subcommand;
use console::style;

use crate::config::Config;

#[derive(Debug, Subcommand)]
pub enum ConfigCmd {
    /// Set a configuration value
    Set {
        /// Key to set (e.g. project, host, api_key)
        key: String,
        /// Value to set
        value: String,
    },

    /// Get a configuration value
    Get {
        /// Key to get
        key: String,
    },

    /// List all configuration values
    List,

    /// Switch the active profile
    SetProfile {
        /// Profile name to activate
        name: String,
    },
}

pub async fn run(cmd: ConfigCmd) -> Result<()> {
    match cmd {
        ConfigCmd::Set { key, value } => {
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
    // Config command logic is primarily tested through config.rs unit tests
    // and integration tests in tests/cli_test.rs
}
