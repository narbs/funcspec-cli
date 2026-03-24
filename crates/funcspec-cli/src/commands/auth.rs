use anyhow::{Context, Result};
use clap::Subcommand;
use console::style;
use funcspec_client::FuncspecClient;

use crate::config::{Config, Profile};

#[derive(Debug, Subcommand)]
pub enum AuthCmd {
    /// Log in to a FuncSpec instance
    Login {
        /// Host URL (default: https://funcspec.net)
        #[arg(long, default_value = "https://funcspec.net")]
        host: String,

        /// API key (or set FUNCSPEC_API_KEY)
        #[arg(long)]
        key: Option<String>,

        /// Profile name
        #[arg(long, default_value = "default")]
        profile: String,
    },

    /// Log out (remove stored credentials)
    Logout {
        /// Profile to remove
        #[arg(long, default_value = "default")]
        profile: String,
    },

    /// Show current auth status
    Status,
}

pub async fn run(cmd: AuthCmd) -> Result<()> {
    match cmd {
        AuthCmd::Login { host, key, profile } => {
            let api_key = match key {
                Some(k) => k,
                None => {
                    // Prompt for key
                    let input = console::Term::stderr()
                        .read_line()
                        .context("Failed to read API key. Pass --key or set FUNCSPEC_API_KEY.")?;
                    eprint!("API key: ");
                    input.trim().to_string()
                }
            };

            // Validate
            eprint!("Validating... ");
            let client = FuncspecClient::new(&host, &api_key)?;
            client.validate_auth().await?;
            eprintln!("{}", style("✓").green().bold());

            // Save
            let mut config = Config::load()?;
            config.profiles.insert(
                profile.clone(),
                Profile {
                    host,
                    api_key,
                    default_project: None,
                },
            );
            config.active_profile = profile.clone();
            config.save()?;

            eprintln!(
                "Logged in as profile {} (saved to {})",
                style(&profile).cyan(),
                Config::config_path()?.display()
            );
            Ok(())
        }

        AuthCmd::Logout { profile } => {
            let mut config = Config::load()?;
            if config.profiles.remove(&profile).is_some() {
                config.save()?;
                eprintln!("Removed profile {}", style(&profile).cyan());
            } else {
                eprintln!("Profile {} not found", style(&profile).yellow());
            }
            Ok(())
        }

        AuthCmd::Status => {
            let config = Config::load()?;
            match config.active_profile() {
                Some(profile) => {
                    eprintln!("Active profile: {}", style(&config.active_profile).cyan());
                    eprintln!("Host: {}", profile.host);
                    eprintln!("API key: {}…", &profile.api_key[..12.min(profile.api_key.len())]);
                    if let Some(ref proj) = profile.default_project {
                        eprintln!("Default project: {proj}");
                    }

                    // Validate
                    eprint!("Connection: ");
                    let client = FuncspecClient::new(&profile.host, &profile.api_key)?;
                    match client.validate_auth().await {
                        Ok(()) => eprintln!("{}", style("✓ authenticated").green()),
                        Err(e) => eprintln!("{} {e}", style("✗").red()),
                    }
                }
                None => {
                    eprintln!(
                        "Not logged in. Run {} to connect.",
                        style("funcspec auth login").cyan()
                    );
                }
            }
            Ok(())
        }
    }
}
