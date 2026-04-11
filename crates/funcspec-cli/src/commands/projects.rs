use anyhow::Result;
use colored::Colorize;
use rust_i18n::t;

use crate::config::Config;
use crate::context::client_and_config;
use crate::output::{self, OutputFormat};

pub enum ProjectsCmd {
    List { json: bool },
    Show { slug: String, json: bool },
    SetDefault { slug: String },
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
            clap::Command::new("set-default")
                .about(t!("cmd.projects.set_default.about").to_string())
                .arg(
                    clap::Arg::new("slug")
                        .required(true)
                        .help(t!("cmd.projects.set_default.slug").to_string()),
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
        Some(("set-default", m)) => ProjectsCmd::SetDefault {
            slug: m.get_one::<String>("slug").unwrap().clone(),
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
    fn build_command_set_default_parses() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["projects", "set-default", "funcspec-cli"])
            .unwrap();
        let sub = m.subcommand_matches("set-default").unwrap();
        assert_eq!(sub.get_one::<String>("slug").unwrap(), "funcspec-cli");
    }
}
