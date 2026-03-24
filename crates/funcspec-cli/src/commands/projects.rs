use anyhow::Result;
use clap::Subcommand;
use console::style;

use crate::config::Config;
use crate::context::client_and_config;
use crate::output;

#[derive(Debug, Subcommand)]
pub enum ProjectsCmd {
    /// List all projects
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show project details
    Show {
        /// Project slug or ID
        slug: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Set default project for commands
    SetDefault {
        /// Project slug (e.g., "funcspec-cli" or "tambit/funcspec-cli")
        slug: String,
    },
}

pub async fn run(cmd: ProjectsCmd) -> Result<()> {
    match cmd {
        ProjectsCmd::List { json } => {
            let (client, _) = client_and_config()?;
            let projects = client.list_projects().await?;

            if json {
                let j = serde_json::to_string_pretty(&projects)?;
                println!("{j}");
            } else {
                output::projects_table(&projects);
            }
            Ok(())
        }

        ProjectsCmd::Show { slug, json } => {
            let (client, _) = client_and_config()?;
            let project = client.get_project(&slug).await?;

            if json {
                let j = serde_json::to_string_pretty(&project)?;
                println!("{j}");
            } else {
                let a = &project.attributes;
                println!("{} {}", style(&a.slug).cyan().bold(), a.name.as_str());
                if let Some(ref desc) = a.description {
                    println!("{desc}");
                }
                println!("Created: {}", a.created_at.format("%Y-%m-%d"));
                println!("Updated: {}", a.updated_at.format("%Y-%m-%d"));
            }
            Ok(())
        }

        ProjectsCmd::SetDefault { slug } => {
            let mut config = Config::load()?;
            if let Some(profile) = config.profiles.get_mut(&config.active_profile.clone()) {
                profile.default_project = Some(slug.clone());
                config.save()?;
                eprintln!("Default project set to {}", style(&slug).cyan());
            } else {
                anyhow::bail!("No active profile. Run `funcspec auth login` first.");
            }
            Ok(())
        }
    }
}
