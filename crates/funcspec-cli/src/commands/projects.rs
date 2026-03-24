use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;

use crate::config::Config;
use crate::context::client_and_config;
use crate::output::{self, OutputFormat};

#[derive(Debug, Subcommand)]
pub enum ProjectsCmd {
    /// List all projects
    List {
        /// Output as JSON (overrides --format)
        #[arg(long)]
        json: bool,
    },

    /// Show project details
    Show {
        /// Project slug or ID
        slug: String,

        /// Output as JSON (overrides --format)
        #[arg(long)]
        json: bool,
    },

    /// Set default project for commands
    SetDefault {
        /// Project slug (e.g., "funcspec-cli" or "tambit/funcspec-cli")
        slug: String,
    },
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
