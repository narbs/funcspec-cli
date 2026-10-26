use anyhow::Result;
use colored::Colorize;
use rust_i18n::t;

use crate::config::Config;
use crate::context::client_and_config;
use crate::output::{self, OutputFormat};
use funcspec_client::models::CreateProjectParams;

pub enum ProjectsCmd {
    List {
        json: bool,
    },
    Show {
        slug: String,
        json: bool,
    },
    Create {
        name: String,
        description: Option<String>,
        slug: Option<String>,
        json: bool,
    },
    SetDefault {
        slug: String,
    },
    Delete {
        slug: String,
        yes: bool,
        json: bool,
    },
}

pub fn build_command() -> clap::Command {
    clap::Command::new("projects")
        .about(t!("cmd.projects.about").to_string())
        .arg_required_else_help(true)
        .subcommand(
            clap::Command::new("list")
                .about(t!("cmd.projects.list.about").to_string())
                .arg(
                    clap::Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help(t!("cmd.projects.list.json").to_string()),
                ),
        )
        .subcommand(
            clap::Command::new("show")
                .about(t!("cmd.projects.show.about").to_string())
                .arg(
                    clap::Arg::new("slug")
                        .required(true)
                        .help(t!("cmd.projects.show.slug").to_string()),
                )
                .arg(
                    clap::Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help(t!("cmd.projects.show.json").to_string()),
                ),
        )
        .subcommand(
            clap::Command::new("create")
                .about("Create a new project")
                .arg(clap::Arg::new("name").required(true).help("Project name"))
                .arg(
                    clap::Arg::new("description")
                        .long("description")
                        .help("Project description"),
                )
                .arg(
                    clap::Arg::new("slug")
                        .long("slug")
                        .help("URL slug (auto-derived from name if omitted)"),
                )
                .arg(
                    clap::Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            clap::Command::new("set-default")
                .about(t!("cmd.projects.set_default.about").to_string())
                .arg(
                    clap::Arg::new("slug")
                        .required(true)
                        .help(t!("cmd.projects.set_default.slug").to_string()),
                ),
        )
        .subcommand(
            clap::Command::new("delete")
                .about("Delete a project (owner/admin only; irreversible)")
                .arg(
                    clap::Arg::new("slug")
                        .required(true)
                        .help("Project ID or slug to delete"),
                )
                .arg(
                    clap::Arg::new("yes")
                        .long("yes")
                        .short('y')
                        .action(clap::ArgAction::SetTrue)
                        .help("Skip the interactive confirmation prompt"),
                )
                .arg(
                    clap::Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
}

pub async fn dispatch(matches: &clap::ArgMatches, format: OutputFormat) -> Result<()> {
    let cmd = match matches.subcommand() {
        Some(("list", m)) => ProjectsCmd::List {
            json: m.get_flag("json"),
        },
        Some(("show", m)) => ProjectsCmd::Show {
            slug: m.get_one::<String>("slug").unwrap().clone(),
            json: m.get_flag("json"),
        },
        Some(("create", m)) => ProjectsCmd::Create {
            name: m.get_one::<String>("name").unwrap().clone(),
            description: m.get_one::<String>("description").cloned(),
            slug: m.get_one::<String>("slug").cloned(),
            json: m.get_flag("json"),
        },
        Some(("set-default", m)) => ProjectsCmd::SetDefault {
            slug: m.get_one::<String>("slug").unwrap().clone(),
        },
        Some(("delete", m)) => ProjectsCmd::Delete {
            slug: m.get_one::<String>("slug").unwrap().clone(),
            yes: m.get_flag("yes"),
            json: m.get_flag("json"),
        },
        _ => {
            build_command().print_help().ok();
            return Ok(());
        }
    };
    run(cmd, format).await
}

pub async fn run(cmd: ProjectsCmd, format: OutputFormat) -> Result<()> {
    match cmd {
        ProjectsCmd::List { json } => {
            let (client, _) = client_and_config()?;
            let projects = client.list_projects().await?;
            let fmt = if json { OutputFormat::Json } else { format };
            output::format_projects(&projects, fmt)?;
            Ok(())
        }

        ProjectsCmd::Show { slug, json } => {
            let (client, _) = client_and_config()?;
            let project = client.get_project(&slug).await?;
            let fmt = if json { OutputFormat::Json } else { format };
            output::format_project_detail(&project, fmt)?;
            Ok(())
        }

        ProjectsCmd::Create {
            name,
            description,
            slug,
            json,
        } => {
            let (client, _) = client_and_config()?;
            let params = CreateProjectParams {
                name,
                description,
                slug,
            };
            let project = client.create_project(&params).await?;
            let fmt = if json { OutputFormat::Json } else { format };
            output::format_project_detail(&project, fmt)?;
            Ok(())
        }

        ProjectsCmd::SetDefault { slug } => {
            let mut config = Config::load()?;
            if let Some(profile) = config.profiles.get_mut(&config.active_profile.clone()) {
                profile.default_project = Some(slug.clone());
                config.save()?;
                eprintln!("Default project set to {}", slug.cyan().bold());
            } else {
                anyhow::bail!("No active profile. Run `funcspec auth login` first.");
            }
            Ok(())
        }

        ProjectsCmd::Delete { slug, yes, json } => {
            let (client, _) = client_and_config()?;

            // Destructive + irreversible: confirm unless -y/--yes was passed.
            // Only prompt on an interactive terminal; a piped/non-TTY run without
            // --yes is refused rather than silently deleting.
            if !yes {
                use std::io::IsTerminal;
                if !std::io::stdin().is_terminal() {
                    anyhow::bail!(
                        "Refusing to delete {} without confirmation. Re-run with --yes to confirm.",
                        slug
                    );
                }
                eprint!(
                    "{} Delete project {}? This is irreversible. Type the slug to confirm: ",
                    "WARNING:".red().bold(),
                    slug.yellow().bold()
                );
                use std::io::Write;
                std::io::stderr().flush().ok();
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if input.trim() != slug {
                    anyhow::bail!("Confirmation did not match; aborting. Nothing was deleted.");
                }
            }

            client.delete_project(&slug).await?;

            if json {
                println!("{}", serde_json::json!({"deleted": slug}));
            } else {
                eprintln!(
                    "{} Deleted project {}",
                    "✓".green().bold(),
                    slug.cyan().bold()
                );
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_command_list_parses() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["projects", "list"]).unwrap();
        let sub = m.subcommand_matches("list").unwrap();
        assert!(!sub.get_flag("json"));
    }

    #[test]
    fn build_command_show_parses() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["projects", "show", "my-slug", "--json"])
            .unwrap();
        let sub = m.subcommand_matches("show").unwrap();
        assert_eq!(sub.get_one::<String>("slug").unwrap(), "my-slug");
        assert!(sub.get_flag("json"));
    }

    #[test]
    fn build_command_delete_parses() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["projects", "delete", "doomed-slug", "--yes"])
            .unwrap();
        let sub = m.subcommand_matches("delete").unwrap();
        assert_eq!(sub.get_one::<String>("slug").unwrap(), "doomed-slug");
        assert!(sub.get_flag("yes"));
    }

    #[test]
    fn build_command_delete_requires_slug() {
        let cmd = build_command();
        let res = cmd.try_get_matches_from(["projects", "delete"]);
        assert!(res.is_err());
    }

    #[test]
    fn build_command_set_default_parses() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["projects", "set-default", "funcspec-cli"])
            .unwrap();
        let sub = m.subcommand_matches("set-default").unwrap();
        assert_eq!(sub.get_one::<String>("slug").unwrap(), "funcspec-cli");
    }
}
