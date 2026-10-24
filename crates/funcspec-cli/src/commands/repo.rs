use anyhow::{Context, Result};
use colored::Colorize;
use comfy_table::{Attribute, Cell, ContentArrangement, Table};
use funcspec_client::{CreateRepositoryParams, UpdateRepositoryParams};

use crate::context::client_and_project;
use crate::output::OutputFormat;

// ---------------------------------------------------------------------------
// Command definitions
// ---------------------------------------------------------------------------

pub enum RepoCmd {
    List {
        json: bool,
    },
    Add {
        github_url: String,
        branch: String,
        name: Option<String>,
        sync_frequency: Option<String>,
    },
    Show {
        repo_id: u64,
        json: bool,
    },
    Sync {
        repo_id: u64,
    },
    SetPrimary {
        repo_id: u64,
    },
    Update {
        repo_id: u64,
        name: Option<String>,
        branch: Option<String>,
        sync_frequency: Option<String>,
    },
    Remove {
        repo_id: u64,
        yes: bool,
    },
}

pub fn build_command() -> clap::Command {
    clap::Command::new("repo")
        .about("Manage project repositories (list, add, sync, set-primary, update, remove)")
        .arg_required_else_help(true)
        .subcommand(
            clap::Command::new("list")
                .about("List repositories linked to the current project")
                .arg(
                    clap::Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            clap::Command::new("add")
                .about("Add a GitHub repository to the project")
                .arg(
                    clap::Arg::new("github_url")
                        .long("url")
                        .short('u')
                        .required(true)
                        .help("GitHub repository URL (e.g. https://github.com/owner/repo)"),
                )
                .arg(
                    clap::Arg::new("branch")
                        .long("branch")
                        .short('b')
                        .default_value("main")
                        .help("Default branch to track"),
                )
                .arg(
                    clap::Arg::new("name")
                        .long("name")
                        .short('n')
                        .help("Display name for the repository (defaults to repo name)"),
                )
                .arg(
                    clap::Arg::new("sync_frequency")
                        .long("sync")
                        .help("Sync frequency: manual, hourly, daily, on_activity"),
                ),
        )
        .subcommand(
            clap::Command::new("show")
                .about("Show details for a repository")
                .arg(
                    clap::Arg::new("repo_id")
                        .required(true)
                        .help("Repository ID"),
                )
                .arg(
                    clap::Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help("Output as JSON"),
                ),
        )
        .subcommand(
            clap::Command::new("sync")
                .about("Trigger a sync for a repository")
                .arg(
                    clap::Arg::new("repo_id")
                        .required(true)
                        .help("Repository ID"),
                ),
        )
        .subcommand(
            clap::Command::new("set-primary")
                .about("Set a repository as the primary repository for the project")
                .arg(
                    clap::Arg::new("repo_id")
                        .required(true)
                        .help("Repository ID"),
                ),
        )
        .subcommand(
            clap::Command::new("update")
                .about("Update repository settings")
                .arg(
                    clap::Arg::new("repo_id")
                        .required(true)
                        .help("Repository ID"),
                )
                .arg(
                    clap::Arg::new("name")
                        .long("name")
                        .short('n')
                        .help("New display name"),
                )
                .arg(
                    clap::Arg::new("branch")
                        .long("branch")
                        .short('b')
                        .help("New default branch"),
                )
                .arg(
                    clap::Arg::new("sync_frequency")
                        .long("sync")
                        .help("New sync frequency: manual, hourly, daily, on_activity"),
                ),
        )
        .subcommand(
            clap::Command::new("remove")
                .about("Remove a repository from the project")
                .arg(
                    clap::Arg::new("repo_id")
                        .required(true)
                        .help("Repository ID"),
                )
                .arg(
                    clap::Arg::new("yes")
                        .long("yes")
                        .short('y')
                        .action(clap::ArgAction::SetTrue)
                        .help("Skip confirmation prompt"),
                ),
        )
}

pub async fn dispatch(matches: &clap::ArgMatches, format: OutputFormat) -> Result<()> {
    let cmd = match matches.subcommand() {
        Some(("list", m)) => RepoCmd::List {
            json: m.get_flag("json"),
        },
        Some(("add", m)) => RepoCmd::Add {
            github_url: m.get_one::<String>("github_url").unwrap().clone(),
            branch: m.get_one::<String>("branch").unwrap().clone(),
            name: m.get_one::<String>("name").cloned(),
            sync_frequency: m.get_one::<String>("sync_frequency").cloned(),
        },
        Some(("show", m)) => RepoCmd::Show {
            repo_id: m
                .get_one::<String>("repo_id")
                .unwrap()
                .parse::<u64>()
                .context("repo_id must be a number")?,
            json: m.get_flag("json"),
        },
        Some(("sync", m)) => RepoCmd::Sync {
            repo_id: m
                .get_one::<String>("repo_id")
                .unwrap()
                .parse::<u64>()
                .context("repo_id must be a number")?,
        },
        Some(("set-primary", m)) => RepoCmd::SetPrimary {
            repo_id: m
                .get_one::<String>("repo_id")
                .unwrap()
                .parse::<u64>()
                .context("repo_id must be a number")?,
        },
        Some(("update", m)) => RepoCmd::Update {
            repo_id: m
                .get_one::<String>("repo_id")
                .unwrap()
                .parse::<u64>()
                .context("repo_id must be a number")?,
            name: m.get_one::<String>("name").cloned(),
            branch: m.get_one::<String>("branch").cloned(),
            sync_frequency: m.get_one::<String>("sync_frequency").cloned(),
        },
        Some(("remove", m)) => RepoCmd::Remove {
            repo_id: m
                .get_one::<String>("repo_id")
                .unwrap()
                .parse::<u64>()
                .context("repo_id must be a number")?,
            yes: m.get_flag("yes"),
        },
        _ => {
            build_command().print_help().ok();
            return Ok(());
        }
    };
    run(cmd, format).await
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub async fn run(cmd: RepoCmd, format: OutputFormat) -> Result<()> {
    match cmd {
        RepoCmd::List { json } => handle_list(json, format).await,
        RepoCmd::Add {
            github_url,
            branch,
            name,
            sync_frequency,
        } => handle_add(&github_url, &branch, name, sync_frequency).await,
        RepoCmd::Show { repo_id, json } => handle_show(repo_id, json, format).await,
        RepoCmd::Sync { repo_id } => handle_sync(repo_id).await,
        RepoCmd::SetPrimary { repo_id } => handle_set_primary(repo_id).await,
        RepoCmd::Update {
            repo_id,
            name,
            branch,
            sync_frequency,
        } => handle_update(repo_id, name, branch, sync_frequency).await,
        RepoCmd::Remove { repo_id, yes } => handle_remove(repo_id, yes).await,
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn handle_list(json: bool, format: OutputFormat) -> Result<()> {
    let (client, project_id) = client_and_project().await?;
    let repos = client.list_repositories(project_id).await?;

    let use_json = json || format == OutputFormat::Json;
    if use_json {
        println!("{}", serde_json::to_string_pretty(&repos)?);
        return Ok(());
    }

    if repos.is_empty() {
        println!("{}", "No repositories linked to this project.".dimmed());
        return Ok(());
    }

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("ID").add_attribute(Attribute::Bold),
        Cell::new("Name").add_attribute(Attribute::Bold),
        Cell::new("GitHub URL").add_attribute(Attribute::Bold),
        Cell::new("Branch").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
        Cell::new("Primary").add_attribute(Attribute::Bold),
        Cell::new("Sync").add_attribute(Attribute::Bold),
    ]);

    for repo in &repos {
        let a = &repo.attributes;
        let primary = if a.is_primary {
            "✓".green().to_string()
        } else {
            String::new()
        };
        table.add_row(vec![
            repo.id.to_string(),
            a.name.clone(),
            a.github_url.clone(),
            a.branch.clone(),
            a.status.clone(),
            primary,
            a.sync_frequency.clone(),
        ]);
    }
    println!("{table}");
    Ok(())
}

async fn handle_add(
    github_url: &str,
    branch: &str,
    name: Option<String>,
    sync_frequency: Option<String>,
) -> Result<()> {
    let (client, project_id) = client_and_project().await?;
    let params = CreateRepositoryParams {
        github_url: github_url.to_string(),
        branch: branch.to_string(),
        name,
        sync_frequency,
    };
    let repo = client.create_repository(project_id, &params).await?;
    let a = &repo.attributes;
    println!(
        "{} Repository {} added (id: {})",
        "✓".green(),
        a.name.bold(),
        repo.id
    );
    if a.is_primary {
        println!("  Set as {}", "primary".cyan());
    }
    Ok(())
}

async fn handle_show(repo_id: u64, json: bool, format: OutputFormat) -> Result<()> {
    let (client, project_id) = client_and_project().await?;
    let repo = client.get_repository(project_id, repo_id).await?;

    let use_json = json || format == OutputFormat::Json;
    if use_json {
        println!("{}", serde_json::to_string_pretty(&repo)?);
        return Ok(());
    }

    let a = &repo.attributes;
    println!("{}", format!("Repository #{}", repo.id).bold());
    println!("  Name:           {}", a.name);
    println!("  URL:            {}", a.github_url);
    println!("  Branch:         {}", a.branch);
    println!("  Status:         {}", colorize_status(&a.status));
    println!(
        "  Primary:        {}",
        if a.is_primary {
            "yes".green().to_string()
        } else {
            "no".to_string()
        }
    );
    println!("  Sync frequency: {}", a.sync_frequency);
    if let Some(ref sha) = a.commit_sha {
        println!("  Last commit:    {}", sha);
    }
    if let Some(ref ts) = a.last_sync_at {
        println!("  Last sync:      {}", ts);
    }
    if let Some(ref msg) = a.error_message {
        println!("  Error:          {}", msg.red());
    }
    Ok(())
}

async fn handle_sync(repo_id: u64) -> Result<()> {
    let (client, project_id) = client_and_project().await?;
    let repo = client.sync_repository(project_id, repo_id).await?;
    println!(
        "{} Sync queued for {} (status: {})",
        "✓".green(),
        repo.attributes.name.bold(),
        repo.attributes.status
    );
    Ok(())
}

async fn handle_set_primary(repo_id: u64) -> Result<()> {
    let (client, project_id) = client_and_project().await?;
    let repo = client.set_primary_repository(project_id, repo_id).await?;
    println!(
        "{} {} is now the primary repository",
        "✓".green(),
        repo.attributes.name.bold()
    );
    Ok(())
}

async fn handle_update(
    repo_id: u64,
    name: Option<String>,
    branch: Option<String>,
    sync_frequency: Option<String>,
) -> Result<()> {
    let (client, project_id) = client_and_project().await?;
    let params = UpdateRepositoryParams {
        name,
        branch,
        sync_frequency,
    };
    let repo = client
        .update_repository(project_id, repo_id, &params)
        .await?;
    println!(
        "{} Repository {} updated",
        "✓".green(),
        repo.attributes.name.bold()
    );
    Ok(())
}

async fn handle_remove(repo_id: u64, yes: bool) -> Result<()> {
    if !yes {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt(format!(
                "Remove repository #{repo_id}? This cannot be undone."
            ))
            .default(false)
            .interact()?;
        if !confirmed {
            println!("Aborted.");
            return Ok(());
        }
    }
    let (client, project_id) = client_and_project().await?;
    client.delete_repository(project_id, repo_id).await?;
    println!("{} Repository #{} removed", "✓".green(), repo_id);
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn colorize_status(status: &str) -> String {
    match status {
        "ready" => status.green().to_string(),
        "syncing" => status.cyan().to_string(),
        "error" => status.red().to_string(),
        _ => status.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_build_command_has_subcommands() {
        let cmd = build_command();
        let subcmds: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(subcmds.contains(&"list"), "missing list");
        assert!(subcmds.contains(&"add"), "missing add");
        assert!(subcmds.contains(&"show"), "missing show");
        assert!(subcmds.contains(&"sync"), "missing sync");
        assert!(subcmds.contains(&"set-primary"), "missing set-primary");
        assert!(subcmds.contains(&"update"), "missing update");
        assert!(subcmds.contains(&"remove"), "missing remove");
    }

    #[test]
    fn t_add_parses_required_args() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(vec![
                "repo",
                "add",
                "--url",
                "https://github.com/owner/repo",
                "--branch",
                "develop",
            ])
            .expect("parse failed");
        let (sub_name, sub_m) = m.subcommand().unwrap();
        assert_eq!(sub_name, "add");
        assert_eq!(
            sub_m.get_one::<String>("github_url").unwrap(),
            "https://github.com/owner/repo"
        );
        assert_eq!(sub_m.get_one::<String>("branch").unwrap(), "develop");
    }

    #[test]
    fn t_remove_yes_flag() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(vec!["repo", "remove", "42", "--yes"])
            .expect("parse failed");
        let (_, sub_m) = m.subcommand().unwrap();
        assert!(sub_m.get_flag("yes"));
    }
}
