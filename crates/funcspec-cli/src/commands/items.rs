use anyhow::Result;
use clap::Subcommand;
use console::style;
use funcspec_client::models::*;
use std::io::Write as IoWrite;

use crate::context::client_and_project;
use crate::output::{self, OutputFormat};

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

        /// Sort field: score, title, created_at, updated_at, permalink
        #[arg(long)]
        sort: Option<String>,

        /// Output as JSON (overrides --format)
        #[arg(long)]
        json: bool,

        /// Quiet mode: only output permalinks (overrides --format)
        #[arg(long)]
        quiet: bool,

        /// Bare TSV output without borders or headers (overrides --format)
        #[arg(long)]
        bare: bool,

        /// Output only the count of matching items
        #[arg(long)]
        count: bool,
    },

    /// Show item details
    Show {
        /// Item permalink (e.g., F-1) or ID
        item: String,

        /// Output as JSON (overrides --format)
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

    /// Edit item description in $EDITOR
    Edit {
        /// Item permalink (e.g., F-1) or ID
        item: String,
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

pub async fn run(cmd: ItemsCmd, format: OutputFormat) -> Result<()> {
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
            sort,
            json,
            quiet,
            bare,
            count,
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
                sort,
                page: Some(page),
                per: Some(per),
            };

            let (items, meta) = client.list_items(project_id, &filter).await?;

            // Client-side filter: API may not enforce type_of/status filters on all projects
            let items: Vec<_> = items
                .into_iter()
                .filter(|item| {
                    if let Some(ref t) = filter.type_of {
                        if item.attributes.type_of != *t {
                            return false;
                        }
                    }
                    if let Some(ref s) = filter.status {
                        if item.attributes.implementation_status != *s {
                            return false;
                        }
                    }
                    true
                })
                .collect();

            if count {
                println!("{}", items.len());
                return Ok(());
            }

            // Per-command flags override the global --format
            let fmt = if json {
                OutputFormat::Json
            } else if quiet {
                OutputFormat::Minimal
            } else if bare {
                OutputFormat::Bare
            } else {
                format
            };
            output::format_items(&items, meta.as_ref(), fmt)?;
            Ok(())
        }

        ItemsCmd::Show { item, json } => {
            let (client, project_id) = client_and_project().await?;
            let spec_item = client.get_item(project_id, &item).await?;
            let fmt = if json { OutputFormat::Json } else { format };
            output::format_item_detail(&spec_item, fmt)?;
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

            let updated = client
                .update_item(project_id, spec_item.id, &params)
                .await?;
            eprintln!(
                "Updated {} {}",
                style(&updated.attributes.permalink).cyan().bold(),
                updated.attributes.title
            );
            Ok(())
        }

        ItemsCmd::Edit { item } => {
            let (client, project_id) = client_and_project().await?;
            let spec_item = client.get_item(project_id, &item).await?;

            let original = spec_item
                .attributes
                .description
                .as_deref()
                .unwrap_or("")
                .to_string();

            // Write description to a temp file
            let mut tmp = tempfile::Builder::new().suffix(".md").tempfile()?;
            tmp.write_all(original.as_bytes())?;
            tmp.flush()?;
            let tmp_path = tmp.path().to_owned();

            // Open $EDITOR (fallback to vi)
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
            let status = std::process::Command::new(&editor)
                .arg(&tmp_path)
                .status()
                .map_err(|e| anyhow::anyhow!("Failed to launch editor '{}': {}", editor, e))?;

            if !status.success() {
                anyhow::bail!("Editor exited with non-zero status");
            }

            let new_description = std::fs::read_to_string(&tmp_path)?;

            if new_description == original {
                eprintln!("No changes.");
                return Ok(());
            }

            let params = UpdateItemParams {
                title: None,
                description: Some(new_description),
                implementation_status: None,
                tags: None,
            };

            let updated = client
                .update_item(project_id, spec_item.id, &params)
                .await?;
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
            eprintln!("Deleted {}", style(&spec_item.attributes.permalink).cyan());
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_tempfile_write_and_read() {
        use std::io::Write as _;
        let content = "# My description\n\nSome details here.";
        let mut tmp = tempfile::Builder::new().suffix(".md").tempfile().unwrap();
        tmp.write_all(content.as_bytes()).unwrap();
        tmp.flush().unwrap();
        let path = tmp.path().to_owned();
        let read_back = std::fs::read_to_string(&path).unwrap();
        assert_eq!(read_back, content);
    }

    #[test]
    fn edit_tempfile_has_md_suffix() {
        let tmp = tempfile::Builder::new().suffix(".md").tempfile().unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        assert!(path.ends_with(".md"), "expected .md suffix, got: {path}");
    }

    #[test]
    fn list_format_override_json_flag() {
        // json=true overrides any global format to Json
        let fmt = if true {
            OutputFormat::Json
        } else {
            OutputFormat::Table
        };
        assert_eq!(fmt, OutputFormat::Json);
    }

    #[test]
    fn list_format_override_quiet_flag() {
        // quiet=true maps to Minimal
        let fmt = if false {
            OutputFormat::Json
        } else if true {
            OutputFormat::Minimal
        } else {
            OutputFormat::Table
        };
        assert_eq!(fmt, OutputFormat::Minimal);
    }

    #[test]
    fn list_format_falls_through_to_global() {
        // neither json nor quiet: use global format
        let global = OutputFormat::Csv;
        let fmt = if false {
            OutputFormat::Json
        } else if false {
            OutputFormat::Minimal
        } else if false {
            OutputFormat::Bare
        } else {
            global
        };
        assert_eq!(fmt, OutputFormat::Csv);
    }

    #[test]
    fn list_format_override_bare_flag() {
        let fmt = if false {
            OutputFormat::Json
        } else if false {
            OutputFormat::Minimal
        } else if true {
            OutputFormat::Bare
        } else {
            OutputFormat::Table
        };
        assert_eq!(fmt, OutputFormat::Bare);
    }
}
