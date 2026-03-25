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

        /// API key (or set FUNCSPEC_API_KEY env var)
        #[arg(long, env = "FUNCSPEC_API_KEY")]
        key: Option<String>,

        /// Profile name to save credentials under
        #[arg(long, default_value = "default")]
        profile: String,
    },

    /// Log out and remove stored credentials
    Logout {
        /// Profile to remove
        #[arg(long, default_value = "default")]
        profile: String,
    },

    /// Show current authentication status
    Status,
}

pub async fn run(cmd: AuthCmd) -> Result<()> {
    match cmd {
        AuthCmd::Login { host, key, profile } => {
            let api_key = match key {
                Some(k) => k,
                None => {
                    eprint!("API key: ");
                    rpassword::read_password()
                        .context("Failed to read API key. Pass --key or set FUNCSPEC_API_KEY.")?
                }
            };

            eprint!("Validating... ");
            let client = FuncspecClient::new(&host, &api_key)?;
            match client.validate_auth().await {
                Ok(user) => {
                    eprintln!("{}", style("✓").green().bold());
                    eprintln!("Authenticated — {}", style(&user.name).cyan());
                }
                Err(e) => {
                    eprintln!("{}", style("✗").red().bold());
                    // Still save if it was a 404 on auth endpoint (endpoint might not exist yet)
                    // but fail on auth errors
                    if matches!(e, funcspec_client::Error::Auth(_)) {
                        return Err(e.into());
                    }
                    eprintln!(
                        "{} Could not validate: {e}",
                        style("warning:").yellow().bold()
                    );
                }
            }

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
                "Saved profile {} to {}",
                style(&profile).cyan(),
                Config::config_path()?.display()
            );
            Ok(())
        }

        AuthCmd::Logout { profile } => {
            let mut config = Config::load()?;
            if config.profiles.remove(&profile).is_some() {
                // If we removed the active profile, clear it
                if config.active_profile == profile {
                    config.active_profile = config
                        .profiles
                        .keys()
                        .next()
                        .cloned()
                        .unwrap_or_else(|| "default".into());
                }
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
                    eprintln!("Profile:  {}", style(&config.active_profile).cyan().bold());
                    eprintln!("Host:     {}", profile.host);
                    let masked = mask_key(&profile.api_key);
                    eprintln!("API key:  {masked}");
                    if let Some(ref proj) = profile.default_project {
                        eprintln!("Project:  {proj}");
                    }

                    eprint!("Status:   ");
                    let client = FuncspecClient::new(&profile.host, &profile.api_key)?;
                    match client.validate_auth().await {
                        Ok(user) => {
                            eprintln!("{} — {}", style("authenticated").green(), user.name,)
                        }
                        Err(e) => eprintln!("{} {e}", style("error:").red()),
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

fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        "*".repeat(key.len())
    } else {
        format!("{}…{}", &key[..4], &key[key.len() - 4..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_key_short() {
        assert_eq!(mask_key("abc"), "***");
    }

    #[test]
    fn mask_key_long() {
        let key = "abcdefghijklmnop";
        let masked = mask_key(key);
        assert!(masked.starts_with("abcd"));
        assert!(masked.ends_with("mnop"));
        assert!(masked.contains('…'));
    }
}
