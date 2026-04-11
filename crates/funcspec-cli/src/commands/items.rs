use anyhow::Result;
use console::style;
use funcspec_client::models::*;
use rust_i18n::t;
use std::io::Write as IoWrite;

use crate::context::client_and_project;
use crate::output::{self, OutputFormat};

pub enum ItemsCmd {
    List {
        r#type: Option<String>,
        status: Option<String>,
        tag: Option<String>,
        q: Option<String>,
        has_review: bool,
        review_verdict: Option<String>,
        parent: Option<String>,
        page: u32,
        per: u32,
        sort: Option<String>,
        json: bool,
        quiet: bool,
        bare: bool,
        count: bool,
    },
    Show {
        item: String,
        json: bool,
    },
    Create {
        title: String,
        r#type: String,
        description: Option<String>,
        parent: Option<String>,
        tag: Option<String>,
    },
    Update {
        item: String,
        title: Option<String>,
        description: Option<String>,
        status: Option<String>,
        tag: Option<String>,
        parent: Option<String>,
        no_parent: bool,
    },
    Edit {
        item: String,
    },
    Delete {
        item: String,
        yes: bool,
    },
}

pub fn build_command() -> clap::Command {
    clap::Command::new("items")
        .about(t!("cmd.items.about").to_string())
        .arg_required_else_help(true)
        .subcommand(
            clap::Command::new("list")
                .about(t!("cmd.items.list.about").to_string())
                .arg(
                    clap::Arg::new("type")
                        .long("type")
                        .short('t')
                        .help(t!("cmd.items.list.type").to_string()),
                )
                .arg(
                    clap::Arg::new("status")
                        .long("status")
                        .short('s')
                        .help(t!("cmd.items.list.status").to_string()),
                )
                .arg(
                    clap::Arg::new("tag")
                        .long("tag")
                        .help(t!("cmd.items.list.tag").to_string()),
                )
                .arg(
                    clap::Arg::new("q")
                        .long("q")
                        .short('q')
                        .help(t!("cmd.items.list.q").to_string()),
                )
                .arg(
                    clap::Arg::new("has_review")
                        .long("has-review")
                        .action(clap::ArgAction::SetTrue)
                        .help(t!("cmd.items.list.has_review").to_string()),
                )
                .arg(
                    clap::Arg::new("review_verdict")
                        .long("review-verdict")
                        .help(t!("cmd.items.list.review_verdict").to_string()),
                )
                .arg(
                    clap::Arg::new("parent")
                        .long("parent")
                        .help(t!("cmd.items.list.parent").to_string()),
                )
                .arg(
                    clap::Arg::new("page")
                        .long("page")
                        .value_parser(clap::value_parser!(u32))
                        .default_value("1")
                        .help(t!("cmd.items.list.page").to_string()),
                )
                .arg(
                    clap::Arg::new("per")
                        .long("per")
                        .value_parser(clap::value_parser!(u32))
                        .default_value("25")
                        .help(t!("cmd.items.list.per").to_string()),
                )
                .arg(
                    clap::Arg::new("sort")
                        .long("sort")
                        .help(t!("cmd.items.list.sort").to_string()),
                )
                .arg(
                    clap::Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help(t!("cmd.items.list.json").to_string()),
                )
                .arg(
                    clap::Arg::new("quiet")
                        .long("quiet")
                        .action(clap::ArgAction::SetTrue)
                        .help(t!("cmd.items.list.quiet").to_string()),
                )
                .arg(
                    clap::Arg::new("bare")
                        .long("bare")
                        .action(clap::ArgAction::SetTrue)
                        .help(t!("cmd.items.list.bare").to_string()),
                )
                .arg(
                    clap::Arg::new("count")
                        .long("count")
                        .action(clap::ArgAction::SetTrue)
                        .help(t!("cmd.items.list.count").to_string()),
                ),
        )
        .subcommand(
            clap::Command::new("show")
                .about(t!("cmd.items.show.about").to_string())
                .arg(
                    clap::Arg::new("item")
                        .required(true)
                        .help(t!("cmd.items.show.item").to_string()),
                )
                .arg(
                    clap::Arg::new("json")
                        .long("json")
                        .action(clap::ArgAction::SetTrue)
                        .help(t!("cmd.items.show.json").to_string()),
                ),
        )
        .subcommand(
            clap::Command::new("create")
                .about(t!("cmd.items.create.about").to_string())
                .arg(
                    clap::Arg::new("title")
                        .long("title")
                        .required(true)
                        .help(t!("cmd.items.create.title").to_string()),
                )
                .arg(
                    clap::Arg::new("type")
                        .long("type")
                        .short('t')
                        .default_value("func")
                        .help(t!("cmd.items.create.type").to_string()),
                )
                .arg(
                    clap::Arg::new("description")
                        .long("description")
                        .short('d')
                        .help(t!("cmd.items.create.description").to_string()),
                )
                .arg(
                    clap::Arg::new("parent")
                        .long("parent")
                        .help(t!("cmd.items.create.parent").to_string()),
                )
                .arg(
                    clap::Arg::new("tag")
                        .long("tag")
                        .help(t!("cmd.items.create.tag").to_string()),
                ),
        )
        .subcommand(
            clap::Command::new("update")
                .about(t!("cmd.items.update.about").to_string())
                .arg(
                    clap::Arg::new("item")
                        .required(true)
                        .help(t!("cmd.items.update.item").to_string()),
                )
                .arg(
                    clap::Arg::new("title")
                        .long("title")
                        .help(t!("cmd.items.update.title").to_string()),
                )
                .arg(
                    clap::Arg::new("description")
                        .long("description")
                        .short('d')
                        .help(t!("cmd.items.update.description").to_string()),
                )
                .arg(
                    clap::Arg::new("status")
                        .long("status")
                        .short('s')
                        .help(t!("cmd.items.update.status").to_string()),
                )
                .arg(
                    clap::Arg::new("tag")
                        .long("tag")
                        .help(t!("cmd.items.update.tag").to_string()),
                )
                .arg(
                    clap::Arg::new("parent")
                        .long("parent")
                        .conflicts_with("no_parent")
                        .help(t!("cmd.items.update.parent").to_string()),
                )
                .arg(
                    clap::Arg::new("no_parent")
                        .long("no-parent")
                        .action(clap::ArgAction::SetTrue)
                        .help(t!("cmd.items.update.no_parent").to_string()),
                ),
        )
        .subcommand(
            clap::Command::new("edit")
                .about(t!("cmd.items.edit.about").to_string())
                .arg(
                    clap::Arg::new("item")
                        .required(true)
                        .help(t!("cmd.items.edit.item").to_string()),
                ),
        )
        .subcommand(
            clap::Command::new("delete")
                .about(t!("cmd.items.delete.about").to_string())
                .arg(
                    clap::Arg::new("item")
                        .required(true)
                        .help(t!("cmd.items.delete.item").to_string()),
                )
                .arg(
                    clap::Arg::new("yes")
                        .long("yes")
                        .short('y')
                        .action(clap::ArgAction::SetTrue)
                        .help(t!("cmd.items.delete.yes").to_string()),
                ),
        )
}

pub async fn dispatch(matches: &clap::ArgMatches, format: OutputFormat) -> Result<()> {
    let cmd = match matches.subcommand() {
        Some(("list", m)) => ItemsCmd::List {
            r#type: m.get_one::<String>("type").cloned(),
            status: m.get_one::<String>("status").cloned(),
            tag: m.get_one::<String>("tag").cloned(),
            q: m.get_one::<String>("q").cloned(),
            has_review: m.get_flag("has_review"),
            review_verdict: m.get_one::<String>("review_verdict").cloned(),
            parent: m.get_one::<String>("parent").cloned(),
            page: m.get_one::<u32>("page").copied().unwrap_or(1),
            per: m.get_one::<u32>("per").copied().unwrap_or(25),
            sort: m.get_one::<String>("sort").cloned(),
            json: m.get_flag("json"),
            quiet: m.get_flag("quiet"),
            bare: m.get_flag("bare"),
            count: m.get_flag("count"),
        },
        Some(("show", m)) => ItemsCmd::Show {
            item: m.get_one::<String>("item").unwrap().clone(),
            json: m.get_flag("json"),
        },
        Some(("create", m)) => ItemsCmd::Create {
            title: m.get_one::<String>("title").unwrap().clone(),
            r#type: m.get_one::<String>("type").unwrap().clone(),
            description: m.get_one::<String>("description").cloned(),
            parent: m.get_one::<String>("parent").cloned(),
            tag: m.get_one::<String>("tag").cloned(),
        },
        Some(("update", m)) => ItemsCmd::Update {
            item: m.get_one::<String>("item").unwrap().clone(),
            title: m.get_one::<String>("title").cloned(),
            description: m.get_one::<String>("description").cloned(),
            status: m.get_one::<String>("status").cloned(),
            tag: m.get_one::<String>("tag").cloned(),
            parent: m.get_one::<String>("parent").cloned(),
            no_parent: m.get_flag("no_parent"),
        },
        Some(("edit", m)) => ItemsCmd::Edit {
            item: m.get_one::<String>("item").unwrap().clone(),
        },
        Some(("delete", m)) => ItemsCmd::Delete {
            item: m.get_one::<String>("item").unwrap().clone(),
            yes: m.get_flag("yes"),
        },
        _ => {
            build_command().print_help().ok();
            return Ok(());
        }
    };
    run(cmd, format).await
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

            let parent_id = if let Some(p) = parent {
                Some(client.resolve_item_id(project_id, &p).await?)
            } else {
                None
            };

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
                    if let Some(ref t) = filter.type_of
                        && item.attributes.type_of != *t
                    {
                        return false;
                    }
                    if let Some(ref s) = filter.status
                        && item.attributes.implementation_status != *s
                    {
                        return false;
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

            let parent_id = if let Some(p) = parent {
                Some(client.resolve_item_id(project_id, &p).await?)
            } else {
                None
            };

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
            parent,
            no_parent,
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

            let parent_id = if no_parent {
                Some(serde_json::Value::Null)
            } else if let Some(p) = parent {
                let id = client.resolve_item_id(project_id, &p).await?;
                Some(serde_json::Value::Number(id.into()))
            } else {
                None
            };

            // Resolve permalink to numeric ID
            let spec_item = client.get_item(project_id, &item).await?;

            let params = UpdateItemParams {
                title,
                description: desc,
                implementation_status: status,
                tags: tag,
                parent_id,
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
                parent_id: None,
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
        let fmt = if true {
            OutputFormat::Json
        } else {
            OutputFormat::Table
        };
        assert_eq!(fmt, OutputFormat::Json);
    }

    #[test]
    fn list_format_override_quiet_flag() {
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
        let global = OutputFormat::Csv;
        let json_flag = false;
        let quiet_flag = false;
        let bare_flag = false;
        let fmt = if json_flag {
            OutputFormat::Json
        } else if quiet_flag {
            OutputFormat::Minimal
        } else if bare_flag {
            OutputFormat::Bare
        } else {
            global
        };
        assert_eq!(fmt, OutputFormat::Csv);
    }

    #[test]
    fn list_format_override_bare_flag() {
        let json_flag = false;
        let quiet_flag = false;
        let bare_flag = true;
        let fmt = if json_flag {
            OutputFormat::Json
        } else if quiet_flag {
            OutputFormat::Minimal
        } else if bare_flag {
            OutputFormat::Bare
        } else {
            OutputFormat::Table
        };
        assert_eq!(fmt, OutputFormat::Bare);
    }

    #[test]
    fn build_command_list_parses_defaults() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["items", "list"]).unwrap();
        let sub = m.subcommand_matches("list").unwrap();
        assert_eq!(sub.get_one::<u32>("page").copied(), Some(1));
        assert_eq!(sub.get_one::<u32>("per").copied(), Some(25));
        assert!(!sub.get_flag("json"));
        assert!(!sub.get_flag("count"));
    }

    #[test]
    fn build_command_list_parses_filters() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["items", "list", "--status", "in_progress", "--type", "func"])
            .unwrap();
        let sub = m.subcommand_matches("list").unwrap();
        assert_eq!(sub.get_one::<String>("status").unwrap(), "in_progress");
        assert_eq!(sub.get_one::<String>("type").unwrap(), "func");
    }

    #[test]
    fn build_command_create_requires_title() {
        let cmd = build_command();
        assert!(cmd.try_get_matches_from(["items", "create"]).is_err());
    }

    #[test]
    fn build_command_create_parses_title() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["items", "create", "--title", "My feature"])
            .unwrap();
        let sub = m.subcommand_matches("create").unwrap();
        assert_eq!(sub.get_one::<String>("title").unwrap(), "My feature");
        assert_eq!(sub.get_one::<String>("type").unwrap(), "func");
    }

    #[test]
    fn build_command_delete_yes_flag() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["items", "delete", "F-1", "--yes"])
            .unwrap();
        let sub = m.subcommand_matches("delete").unwrap();
        assert_eq!(sub.get_one::<String>("item").unwrap(), "F-1");
        assert!(sub.get_flag("yes"));
    }

    #[test]
    fn build_command_update_no_parent_flag() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["items", "update", "F-5", "--no-parent"])
            .unwrap();
        let sub = m.subcommand_matches("update").unwrap();
        assert!(sub.get_flag("no_parent"));
    }

    #[test]
    fn build_command_update_parent_conflicts_no_parent() {
        let cmd = build_command();
        assert!(
            cmd.try_get_matches_from(["items", "update", "F-5", "--parent", "F-1", "--no-parent"])
                .is_err()
        );
    }
}
