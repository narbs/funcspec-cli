use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use comfy_table::{Attribute, Cell, ContentArrangement, Table};
use funcspec_client::CreateSnapshotParams;

use crate::context::client_and_project;
use crate::output::OutputFormat;

// ---------------------------------------------------------------------------
// Command definitions
// ---------------------------------------------------------------------------

#[derive(Debug, Subcommand)]
pub enum SnapshotsCmd {
    /// List all snapshots for the current project
    List {
        /// Output as JSON (overrides --format)
        #[arg(long)]
        json: bool,
    },

    /// Create a new snapshot
    Create {
        /// Snapshot name (e.g. "pre-v2-refactor")
        #[arg(long, short)]
        name: String,

        /// Optional description
        #[arg(long, short)]
        description: Option<String>,
    },

    /// Show snapshot details and item summary
    Show {
        /// Snapshot ID or name
        identifier: String,

        /// Output as JSON (overrides --format)
        #[arg(long)]
        json: bool,
    },

    /// Restore project to a snapshot (destructive!)
    Restore {
        /// Snapshot ID or name
        identifier: String,

        /// Skip confirmation prompt
        #[arg(long, short)]
        yes: bool,
    },

    /// Show what changed since a snapshot was taken
    Diff {
        /// Snapshot ID or name
        identifier: String,

        /// Output as JSON (overrides --format)
        #[arg(long)]
        json: bool,
    },

    /// Delete a snapshot
    Delete {
        /// Snapshot ID or name
        identifier: String,

        /// Skip confirmation prompt
        #[arg(long, short)]
        yes: bool,
    },
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub async fn run(cmd: SnapshotsCmd, format: OutputFormat) -> Result<()> {
    match cmd {
        SnapshotsCmd::List { json } => handle_list(json, format).await,
        SnapshotsCmd::Create { name, description } => handle_create(&name, description).await,
        SnapshotsCmd::Show { identifier, json } => handle_show(&identifier, json, format).await,
        SnapshotsCmd::Restore { identifier, yes } => handle_restore(&identifier, yes).await,
        SnapshotsCmd::Diff { identifier, json } => handle_diff(&identifier, json, format).await,
        SnapshotsCmd::Delete { identifier, yes } => handle_delete(&identifier, yes).await,
    }
}

// ---------------------------------------------------------------------------
// Identifier resolution — try numeric ID first, then name match
// ---------------------------------------------------------------------------

async fn resolve_snapshot_id(
    client: &funcspec_client::FuncspecClient,
    project_id: u64,
    identifier: &str,
) -> Result<u64> {
    // Fast path: numeric ID
    if let Ok(id) = identifier.parse::<u64>() {
        return Ok(id);
    }

    // Slow path: list and match by name
    let snapshots = client
        .list_snapshots(project_id)
        .await
        .context("Failed to list snapshots")?;

    let matches: Vec<_> = snapshots
        .iter()
        .filter(|s| s.attributes.name == identifier)
        .collect();

    match matches.len() {
        0 => anyhow::bail!(
            "No snapshot found with name or ID {:?}. Run `funcspec snapshots list` to see available snapshots.",
            identifier
        ),
        1 => Ok(matches[0].id),
        _ => {
            let ids: Vec<String> = matches.iter().map(|s| s.id.to_string()).collect();
            anyhow::bail!(
                "Ambiguous: multiple snapshots match {:?} (IDs: {}). Use the numeric ID instead.",
                identifier,
                ids.join(", ")
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn handle_list(json: bool, format: OutputFormat) -> Result<()> {
    let (client, project_id) = client_and_project().await?;
    let snapshots = client
        .list_snapshots(project_id)
        .await
        .context("Failed to list snapshots")?;

    if json {
        println!("{}", serde_json::to_string_pretty(&snapshots)?);
        return Ok(());
    }

    match format.resolve() {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&snapshots)?);
        }
        OutputFormat::Minimal | OutputFormat::Bare => {
            for s in &snapshots {
                println!(
                    "{}\t{}\t{}",
                    s.id,
                    s.attributes.name,
                    s.attributes.created_at.format("%Y-%m-%d")
                );
            }
        }
        _ => {
            if snapshots.is_empty() {
                eprintln!(
                    "No snapshots found. Create one with `funcspec snapshots create --name <name>`."
                );
                return Ok(());
            }
            let mut table = Table::new();
            table
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(vec![
                    Cell::new("ID").add_attribute(Attribute::Bold),
                    Cell::new("Name").add_attribute(Attribute::Bold),
                    Cell::new("Created").add_attribute(Attribute::Bold),
                    Cell::new("Items").add_attribute(Attribute::Bold),
                ]);

            for s in &snapshots {
                let a = &s.attributes;
                table.add_row(vec![
                    Cell::new(s.id.to_string()),
                    Cell::new(&a.name),
                    Cell::new(a.created_at.format("%Y-%m-%d %H:%M").to_string()),
                    Cell::new(a.spec_items.len().to_string()),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

async fn handle_create(name: &str, description: Option<String>) -> Result<()> {
    let (client, project_id) = client_and_project().await?;
    let params = CreateSnapshotParams {
        name: name.to_string(),
        description,
    };
    let snapshot = client
        .create_snapshot(project_id, &params)
        .await
        .context("Failed to create snapshot")?;

    let item_count = snapshot.attributes.spec_items.len();
    eprintln!(
        "{} Snapshot {} created (ID: {}, {} items captured)",
        "✓".green().bold(),
        snapshot.attributes.name.cyan().bold(),
        snapshot.id,
        item_count
    );
    Ok(())
}

async fn handle_show(identifier: &str, json: bool, format: OutputFormat) -> Result<()> {
    let (client, project_id) = client_and_project().await?;
    let snapshot_id = resolve_snapshot_id(&client, project_id, identifier).await?;
    let snapshot = client
        .get_snapshot(project_id, snapshot_id)
        .await
        .with_context(|| format!("Snapshot {identifier:?} not found"))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&snapshot)?);
        return Ok(());
    }

    match format.resolve() {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&snapshot)?);
        }
        _ => {
            let a = &snapshot.attributes;
            println!("{}", "Snapshot".bold());
            println!("  {} {}", "ID:".bold(), snapshot.id);
            println!("  {} {}", "Name:".bold(), a.name.cyan());
            if let Some(ref desc) = a.description {
                println!("  {} {}", "Description:".bold(), desc);
            }
            println!(
                "  {} {}",
                "Created:".bold(),
                a.created_at.format("%Y-%m-%d %H:%M:%S UTC")
            );
            println!("  {} {}", "Items:".bold(), a.spec_items.len());
            println!();

            if !a.spec_items.is_empty() {
                println!("{}", "Item summary:".bold());
                let func_count = a
                    .spec_items
                    .iter()
                    .filter(|i| i.attributes.type_of == funcspec_client::ItemType::Functional)
                    .count();
                let tech_count = a.spec_items.len() - func_count;
                println!(
                    "  {} functional, {} technical",
                    func_count.to_string().cyan(),
                    tech_count.to_string().cyan()
                );
            }
        }
    }
    Ok(())
}

async fn handle_restore(identifier: &str, yes: bool) -> Result<()> {
    let (client, project_id) = client_and_project().await?;
    let snapshot_id = resolve_snapshot_id(&client, project_id, identifier).await?;

    // Show snapshot info before confirming
    let snapshot = client
        .get_snapshot(project_id, snapshot_id)
        .await
        .with_context(|| format!("Snapshot {identifier:?} not found"))?;

    let a = &snapshot.attributes;
    eprintln!(
        "{} Snapshot: {} (ID: {}, {} items, created {})",
        "→".yellow(),
        a.name.cyan().bold(),
        snapshot.id,
        a.spec_items.len(),
        a.created_at.format("%Y-%m-%d %H:%M")
    );
    eprintln!(
        "{} This will OVERWRITE the current project state with the snapshot contents.",
        "warning:".yellow().bold()
    );

    if !yes {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt("Restore this snapshot? This is destructive and cannot be undone")
            .default(false)
            .interact()
            .context("Prompt error")?;

        if !confirmed {
            eprintln!("Restore cancelled.");
            return Ok(());
        }
    }

    client
        .restore_snapshot(project_id, snapshot_id)
        .await
        .context("Failed to restore snapshot")?;

    eprintln!(
        "{} Restored snapshot {} (ID: {})",
        "✓".green().bold(),
        a.name.cyan().bold(),
        snapshot.id
    );
    Ok(())
}

async fn handle_diff(identifier: &str, json: bool, format: OutputFormat) -> Result<()> {
    let (client, project_id) = client_and_project().await?;
    let snapshot_id = resolve_snapshot_id(&client, project_id, identifier).await?;
    let diff = client
        .diff_snapshot(project_id, snapshot_id)
        .await
        .with_context(|| format!("Failed to get diff for snapshot {identifier:?}"))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&diff)?);
        return Ok(());
    }

    match format.resolve() {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&diff)?);
        }
        _ => {
            let total = diff.added.len() + diff.removed.len() + diff.modified.len();
            if total == 0 {
                println!("{}", "No changes since snapshot.".green());
                return Ok(());
            }

            println!(
                "{} {} added, {} removed, {} modified",
                "Diff:".bold(),
                diff.added.len().to_string().green(),
                diff.removed.len().to_string().red(),
                diff.modified.len().to_string().yellow()
            );
            println!();

            if !diff.added.is_empty() {
                println!("{}", "Added items:".green().bold());
                for item in &diff.added {
                    println!(
                        "  {} {} — {}",
                        "+".green().bold(),
                        item.attributes.permalink.cyan(),
                        item.attributes.title
                    );
                }
                println!();
            }

            if !diff.removed.is_empty() {
                println!("{}", "Removed items:".red().bold());
                for item in &diff.removed {
                    println!(
                        "  {} {} — {}",
                        "-".red().bold(),
                        item.attributes.permalink.cyan(),
                        item.attributes.title
                    );
                }
                println!();
            }

            if !diff.modified.is_empty() {
                println!("{}", "Modified items:".yellow().bold());
                for entry in &diff.modified {
                    let before = &entry.before.attributes;
                    let after = &entry.after.attributes;
                    println!(
                        "  {} {} — {}",
                        "~".yellow().bold(),
                        after.permalink.cyan(),
                        after.title
                    );
                    if before.title != after.title {
                        println!(
                            "      title: {} → {}",
                            before.title.red(),
                            after.title.green()
                        );
                    }
                    if before.description != after.description {
                        println!("      description changed");
                    }
                    if before.implementation_status != after.implementation_status {
                        println!(
                            "      status: {} → {}",
                            before.implementation_status.to_string().red(),
                            after.implementation_status.to_string().green()
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

async fn handle_delete(identifier: &str, yes: bool) -> Result<()> {
    let (client, project_id) = client_and_project().await?;
    let snapshot_id = resolve_snapshot_id(&client, project_id, identifier).await?;

    let snapshot = client
        .get_snapshot(project_id, snapshot_id)
        .await
        .with_context(|| format!("Snapshot {identifier:?} not found"))?;

    let a = &snapshot.attributes;
    eprintln!(
        "{} Snapshot: {} (ID: {}, {} items, created {})",
        "→".yellow(),
        a.name.cyan().bold(),
        snapshot.id,
        a.spec_items.len(),
        a.created_at.format("%Y-%m-%d %H:%M")
    );

    if !yes {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt("Delete this snapshot?")
            .default(false)
            .interact()
            .context("Prompt error")?;

        if !confirmed {
            eprintln!("Delete cancelled.");
            return Ok(());
        }
    }

    client
        .delete_snapshot(project_id, snapshot_id)
        .await
        .context("Failed to delete snapshot")?;

    eprintln!(
        "{} Deleted snapshot {} (ID: {})",
        "✓".green().bold(),
        a.name.cyan().bold(),
        snapshot.id
    );
    Ok(())
}
