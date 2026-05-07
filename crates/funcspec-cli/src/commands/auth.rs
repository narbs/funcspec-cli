use anyhow::{Context, Result};
use console::style;
use funcspec_client::FuncspecClient;
use rust_i18n::t;

use crate::config::{Config, Profile};

pub enum AuthCmd {
    Login {
        host: String,
        key: Option<String>,
        profile: String,
    },
    Logout {
        profile: String,
    },
    Status,
}

pub fn build_command() -> clap::Command {
    clap::Command::new("auth")
        .about(t!("cmd.auth.about").to_string())
        .arg_required_else_help(true)
        .subcommand(
            clap::Command::new("login")
                .about(t!("cmd.auth.login.about").to_string())
                .arg(
                    clap::Arg::new("host")
                        .long("host")
                        .default_value("https://funcspec.net")
                        .help(t!("cmd.auth.login.host").to_string()),
                )
                .arg(
                    clap::Arg::new("key")
                        .long("key")
                        .env("FUNCSPEC_API_KEY")
                        .help(t!("cmd.auth.login.key").to_string()),
                )
                .arg(
                    clap::Arg::new("profile")
                        .long("profile")
                        .default_value("default")
                        .help(t!("cmd.auth.login.profile").to_string()),
                ),
        )
        .subcommand(
            clap::Command::new("logout")
                .about(t!("cmd.auth.logout.about").to_string())
                .arg(
                    clap::Arg::new("profile")
                        .long("profile")
                        .default_value("default")
                        .help(t!("cmd.auth.logout.profile").to_string()),
                ),
        )
        .subcommand(clap::Command::new("status").about(t!("cmd.auth.status.about").to_string()))
}

pub async fn dispatch(matches: &clap::ArgMatches) -> Result<()> {
    let cmd = match matches.subcommand() {
        Some(("login", m)) => AuthCmd::Login {
            host: m.get_one::<String>("host").unwrap().clone(),
            key: m.get_one::<String>("key").cloned(),
            profile: m.get_one::<String>("profile").unwrap().clone(),
        },
        Some(("logout", m)) => AuthCmd::Logout {
            profile: m.get_one::<String>("profile").unwrap().clone(),
        },
        Some(("status", _)) => AuthCmd::Status,
        _ => {
            build_command().print_help().ok();
            return Ok(());
        }
    };
    run(cmd).await
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
    use std::sync::{Mutex, MutexGuard};

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    struct EnvGuard<'a> {
        _lock: MutexGuard<'a, ()>,
        saved: Vec<(String, Option<String>)>,
    }

    impl<'a> EnvGuard<'a> {
        fn unset(keys: &[&str]) -> Self {
            let lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
            let saved = keys
                .iter()
                .map(|k| {
                    let v = std::env::var(k).ok();
                    // SAFETY: we hold ENV_MUTEX exclusively across all env-sensitive tests
                    unsafe { std::env::remove_var(k) };
                    (k.to_string(), v)
                })
                .collect();
            Self { _lock: lock, saved }
        }
    }

    impl Drop for EnvGuard<'_> {
        fn drop(&mut self) {
            for (k, v) in &self.saved {
                match v {
                    Some(val) => unsafe { std::env::set_var(k, val) },
                    None => unsafe { std::env::remove_var(k) },
                }
            }
        }
    }

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

    #[test]
    fn build_command_login_parses_defaults() {
        let _env = EnvGuard::unset(&["FUNCSPEC_API_KEY"]);
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["auth", "login"]).unwrap();
        let sub = m.subcommand_matches("login").unwrap();
        assert_eq!(
            sub.get_one::<String>("host").unwrap(),
            "https://funcspec.net"
        );
        assert_eq!(sub.get_one::<String>("profile").unwrap(), "default");
        assert!(sub.get_one::<String>("key").is_none());
    }

    #[test]
    fn build_command_login_parses_custom() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from([
                "auth",
                "login",
                "--host",
                "https://self.host",
                "--profile",
                "work",
            ])
            .unwrap();
        let sub = m.subcommand_matches("login").unwrap();
        assert_eq!(sub.get_one::<String>("host").unwrap(), "https://self.host");
        assert_eq!(sub.get_one::<String>("profile").unwrap(), "work");
    }

    #[test]
    fn build_command_logout_default_profile() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["auth", "logout"]).unwrap();
        let sub = m.subcommand_matches("logout").unwrap();
        assert_eq!(sub.get_one::<String>("profile").unwrap(), "default");
    }
}
