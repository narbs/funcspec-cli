use anyhow::Result;
use clap::Subcommand;
use console::style;
use funcspec_client::models::*;

use crate::context::{client_and_project, OutputMode};
use crate::output;

#[derive(Debug, Subcommand)]
pub enum ItemsCmd {
    /// List spec items
    List {
        /// Filter by type: func or tech
        #[arg(long, short = 't')]
        r#type: Option<String>,

        /// Filter by implementation status
        #[arg(long, short = 's')]
        status: Option<String>,

        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,

        /// Search query
        #[arg(long, short)]
        q: Option<String>,

        /// Filter: only items with reviews
        #[arg(long)]
        has_review: bool,

        /// Filter by review verdict
        #[arg(long)]
        review_verdict: Option<String>,

        /// Filter by parent item (permalink or ID)
        #[arg(long)]
        parent: Option<String>,

        /// Page number
        #[arg(long, default_value = "1")]
        page: u32,

        /// Items per page
        #[arg(long, default_value = "25")]
        per: u32,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Quiet mode: only output permalinks
        #[arg(long)]
        quiet: bool,
    },

    /// Show item details
    Show {
        /// Item permalink (e.g., F-1) or ID
        item: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Create a new spec item
    Create {
        /// Item title
        #[arg(long)]
        title: String,

        /// Type: func or tech
        #[arg(long, short = 't', default_value = "func")]
        r#type: String,

        /// Description (reads from stdin if "-")
        #[arg(long, short)]
        description: Option<String>,

        /// Parent item permalink or ID
        #[arg(long)]
        parent: Option<String>,

        /// Tags (comma-separated)
        #[arg(long)]
        tag: Option<String>,
    },

    /// Update a spec item
    Update {
        /// Item permalink (e.g., F-377) or ID
        item: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New description (reads from stdin if "-")
        #[arg(long, short)]
        description: Option<String>,

        /// New implementation status
        #[arg(long, short = 's')]
        status: Option<String>,

        /// Tags (comma-separated, replaces existing)
        #[arg(long)]
        tag: Option<String>,
    },

    /// Delete a spec item
    Delete {
        /// Item permalink or ID
        item: String,

        /// Skip confirmation
        #[arg(long, short)]
        yes: bool,
    },
}

pub async fn run(cmd: ItemsCmd) -> Result<()> {
    match cmd {
        ItemsCmd::List {
            r#type,
            status,
            tag,
            q,
            has_review,
            review_verdict,
            parent,
            page,
            per,
            json,
            quiet,
        } => {
            let (client, project_id) = client_and_project().await?;

            let type_of = r#type.map(|t| match t.as_str() {
                "func" | "functional" => ItemType::Functional,
                "tech" | "technical" => ItemType::Technical,
                _ => ItemType::Functional,
            });

            let impl_status = status.map(|s| match s.as_str() {
                "implemented" => ImplementationStatus::Implemented,
                "in_progress" => ImplementationStatus::InProgress,
                _ => ImplementationStatus::NotStarted,
            });

            // TODO: resolve parent permalink to ID if needed
            let parent_id = parent.and_then(|p| p.parse::<u64>().ok());

            let filter = ItemFilter {
                type_of,
                status: impl_status,
                tag,
                q,
                has_review: if has_review { Some(true) } else { None },
                review_verdict,
                parent_id,
                page: Some(page),
                per: Some(per),
            };

            let (items, meta) = client.list_items(project_id, &filter).await?;

            let mode = OutputMode::from_flags(json, quiet);
            match mode {
                OutputMode::Json => output::items_json(&items),
                OutputMode::Quiet => output::items_quiet(&items),
                OutputMode::Table => output::items_table(&items, meta.as_ref()),
            }
            Ok(())
        }

        ItemsCmd::Show { item, json } => {
            let (client, project_id) = client_and_project().await?;
            let spec_item = client.get_item(project_id, &item).await?;

            if json {
                let j = serde_json::to_string_pretty(&spec_item)?;
                println!("{j}");
            } else {
                output::item_detail(&spec_item);
            }
            Ok(())
        }

        ItemsCmd::Create {
            title,
            r#type,
            description,
            parent,
            tag,
        } => {
            let (client, project_id) = client_and_project().await?;

            let desc = match description.as_deref() {
                Some("-") => {
                    let mut buf = String::new();
                    std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
                    Some(buf)
                }
                other => other.map(String::from),
            };

            let type_of = match r#type.as_str() {
                "tech" | "technical" => "technical",
                _ => "functional",
            };

            let parent_id = parent.and_then(|p| p.parse::<u64>().ok());

            let params = CreateItemParams {
                title: title.clone(),
                type_of: type_of.into(),
                description: desc,
                parent_id,
                tags: tag,
            };

            let created = client.create_item(project_id, &params).await?;
            eprintln!(
                "Created {} {}",
                style(&created.attributes.permalink).cyan().bold(),
                created.attributes.title
            );
            Ok(())
        }

        ItemsCmd::Update {
            item,
            title,
            description,
            status,
            tag,
        } => {
            let (client, project_id) = client_and_project().await?;

            let desc = match description.as_deref() {
                Some("-") => {
                    let mut buf = String::new();
                    std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
                    Some(buf)
                }
                other => other.map(String::from),
            };

            // Resolve permalink to numeric ID
            let spec_item = client.get_item(project_id, &item).await?;

            let params = UpdateItemParams {
                title,
                description: desc,
                implementation_status: status,
                tags: tag,
            };

            let updated = client.update_item(project_id, spec_item.id, &params).await?;
            eprintln!(
                "Updated {} {}",
                style(&updated.attributes.permalink).cyan().bold(),
                updated.attributes.title
            );
            Ok(())
        }

        ItemsCmd::Delete { item, yes } => {
            let (client, project_id) = client_and_project().await?;
            let spec_item = client.get_item(project_id, &item).await?;

            if !yes {
                eprint!(
                    "Delete {} {}? [y/N] ",
                    spec_item.attributes.permalink, spec_item.attributes.title
                );
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    eprintln!("Cancelled.");
                    return Ok(());
                }
            }

            client.delete_item(project_id, spec_item.id).await?;
            eprintln!(
                "Deleted {}",
                style(&spec_item.attributes.permalink).cyan()
            );
            Ok(())
        }
    }
}
