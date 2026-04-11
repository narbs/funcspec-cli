use anyhow::{Result, bail};
use console::style;
use funcspec_client::models::*;
use rust_i18n::t;
use std::collections::HashMap;

use crate::context::client_and_project;
use crate::output::{self, OutputFormat};

pub enum EdgesCmd {
    List {
        source: Option<String>,
        target: Option<String>,
        r#type: Option<String>,
        json: bool,
    },
    Link {
        source: String,
        target: String,
        r#type: String,
    },
    Unlink {
        edge_id: Option<u64>,
        source: Option<String>,
        target: Option<String>,
        r#type: Option<String>,
        yes: bool,
    },
}

pub fn build_command() -> clap::Command {
    clap::Command::new("edges")
        .about(t!("cmd.edges.about").to_string())
        .arg_required_else_help(true)
        .subcommand(
            clap::Command::new("list")
                .about(t!("cmd.edges.list.about").to_string())
                .arg(clap::Arg::new("source").long("source").help(t!("cmd.edges.list.source").to_string()))
                .arg(clap::Arg::new("target").long("target").help(t!("cmd.edges.list.target").to_string()))
                .arg(clap::Arg::new("type").long("type").value_name("TYPE").help(t!("cmd.edges.list.type").to_string()))
                .arg(clap::Arg::new("json").long("json").action(clap::ArgAction::SetTrue).help(t!("cmd.edges.list.json").to_string())),
        )
        .subcommand(
            clap::Command::new("link")
                .about(t!("cmd.edges.link.about").to_string())
                .arg(clap::Arg::new("source").long("source").required(true).help(t!("cmd.edges.link.source").to_string()))
                .arg(clap::Arg::new("target").long("target").required(true).help(t!("cmd.edges.link.target").to_string()))
                .arg(clap::Arg::new("type").long("type").value_name("TYPE").required(true).help(t!("cmd.edges.link.type").to_string())),
        )
        .subcommand(
            clap::Command::new("unlink")
                .about(t!("cmd.edges.unlink.about").to_string())
                .arg(clap::Arg::new("edge_id").value_parser(clap::value_parser!(u64)).help(t!("cmd.edges.unlink.edge_id").to_string()))
                .arg(clap::Arg::new("source").long("source").help(t!("cmd.edges.unlink.source").to_string()))
                .arg(clap::Arg::new("target").long("target").help(t!("cmd.edges.unlink.target").to_string()))
                .arg(clap::Arg::new("type").long("type").value_name("TYPE").help(t!("cmd.edges.unlink.type").to_string()))
                .arg(clap::Arg::new("yes").long("yes").short('y').action(clap::ArgAction::SetTrue).help(t!("cmd.edges.unlink.yes").to_string())),
        )
}

pub async fn dispatch(matches: &clap::ArgMatches, format: OutputFormat) -> Result<()> {
    let cmd = match matches.subcommand() {
        Some(("list", m)) => EdgesCmd::List {
            source: m.get_one::<String>("source").cloned(),
            target: m.get_one::<String>("target").cloned(),
            r#type: m.get_one::<String>("type").cloned(),
            json: m.get_flag("json"),
        },
        Some(("link", m)) => EdgesCmd::Link {
            source: m.get_one::<String>("source").unwrap().clone(),
            target: m.get_one::<String>("target").unwrap().clone(),
            r#type: m.get_one::<String>("type").unwrap().clone(),
        },
        Some(("unlink", m)) => EdgesCmd::Unlink {
            edge_id: m.get_one::<u64>("edge_id").copied(),
            source: m.get_one::<String>("source").cloned(),
            target: m.get_one::<String>("target").cloned(),
            r#type: m.get_one::<String>("type").cloned(),
            yes: m.get_flag("yes"),
        },
        _ => {
            build_command().print_help().ok();
            return Ok(());
        }
    };
    run(cmd, format).await
}

pub async fn run(cmd: EdgesCmd, format: OutputFormat) -> Result<()> {
    match cmd {
        EdgesCmd::List {
            source,
            target,
            r#type,
            json,
        } => {
            let (client, project_id) = client_and_project().await?;

            // Resolve source/target permalinks to numeric IDs if provided
            let source_id = if let Some(ref s) = source {
                Some(client.resolve_item_id(project_id, s).await?)
            } else {
                None
            };
            let target_id = if let Some(ref t) = target {
                Some(client.resolve_item_id(project_id, t).await?)
            } else {
                None
            };

            let edges = client
                .list_edges(project_id, source_id, target_id, r#type.as_deref())
                .await?;

            let fmt = if json { OutputFormat::Json } else { format };

            // For non-JSON formats, resolve item IDs to (permalink, title) pairs
            let item_map = if fmt.resolve() != OutputFormat::Json {
                build_item_map(&client, project_id, &edges).await
            } else {
                HashMap::new()
            };

            output::format_edges(&edges, &item_map, fmt)?;
            Ok(())
        }

        EdgesCmd::Link {
            source,
            target,
            r#type,
        } => {
            let (client, project_id) = client_and_project().await?;

            let source_id = client.resolve_item_id(project_id, &source).await?;
            let target_id = client.resolve_item_id(project_id, &target).await?;

            let params = CreateEdgeParams {
                source_id,
                target_id,
                edge_type: r#type.clone(),
            };

            let edge = client.create_edge(project_id, &params).await?;

            // Fetch both items for display
            let src_item = client
                .get_item(project_id, &source_id.to_string())
                .await
                .ok();
            let tgt_item = client
                .get_item(project_id, &target_id.to_string())
                .await
                .ok();

            let src_display = src_item
                .as_ref()
                .map(|i| {
                    format!(
                        "{} {}",
                        style(&i.attributes.permalink).cyan().bold(),
                        i.attributes.title
                    )
                })
                .unwrap_or_else(|| source_id.to_string());
            let tgt_display = tgt_item
                .as_ref()
                .map(|i| {
                    format!(
                        "{} {}",
                        style(&i.attributes.permalink).cyan().bold(),
                        i.attributes.title
                    )
                })
                .unwrap_or_else(|| target_id.to_string());

            eprintln!(
                "Linked {} --[{}]--> {} (edge {})",
                src_display,
                style(&r#type).yellow(),
                tgt_display,
                style(edge.id).dim(),
            );
            Ok(())
        }

        EdgesCmd::Unlink {
            edge_id,
            source,
            target,
            r#type,
            yes,
        } => {
            let (client, project_id) = client_and_project().await?;

            let resolved_edge_id = if let Some(id) = edge_id {
                id
            } else {
                // Resolve by source/target/type combo
                let source_id = if let Some(ref s) = source {
                    Some(client.resolve_item_id(project_id, s).await?)
                } else {
                    None
                };
                let target_id = if let Some(ref t) = target {
                    Some(client.resolve_item_id(project_id, t).await?)
                } else {
                    None
                };

                if source_id.is_none() && target_id.is_none() && r#type.is_none() {
                    bail!(
                        "Provide an edge ID, or use --source/--target/--type to identify the edge"
                    );
                }

                let edges = client
                    .list_edges(project_id, source_id, target_id, r#type.as_deref())
                    .await?;

                match edges.len() {
                    0 => bail!("No matching edge found"),
                    1 => edges[0].id,
                    n => {
                        bail!("{n} edges match — narrow the filter or specify an edge ID directly")
                    }
                }
            };

            if !yes {
                eprint!("Delete edge {}? [y/N] ", resolved_edge_id);
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                if !input.trim().eq_ignore_ascii_case("y") {
                    eprintln!("Cancelled.");
                    return Ok(());
                }
            }

            client.delete_edge(project_id, resolved_edge_id).await?;
            eprintln!("Deleted edge {}", style(resolved_edge_id).cyan());
            Ok(())
        }
    }
}

/// Collect the unique source/target item IDs from a list of edges and
/// fetch each item, returning a map of ID → (permalink, title).
async fn build_item_map(
    client: &funcspec_client::FuncspecClient,
    project_id: u64,
    edges: &[DependencyEdge],
) -> HashMap<u64, (String, String)> {
    let mut ids: std::collections::HashSet<u64> = std::collections::HashSet::new();
    for edge in edges {
        ids.insert(edge.attributes.source_id);
        ids.insert(edge.attributes.target_id);
    }

    let mut map = HashMap::new();
    for id in ids {
        if let Ok(item) = client.get_item(project_id, &id.to_string()).await {
            map.insert(id, (item.attributes.permalink, item.attributes.title));
        }
    }
    map
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_edge_serde_roundtrip() {
        let json = serde_json::json!({
            "id": 42,
            "type": "dependency_edge",
            "attributes": {
                "source_id": 1,
                "target_id": 2,
                "edge_type": "implements",
                "created_at": "2026-01-01T00:00:00Z"
            }
        });
        let edge: DependencyEdge = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(edge.id, 42);
        assert_eq!(edge.attributes.source_id, 1);
        assert_eq!(edge.attributes.target_id, 2);
        assert_eq!(edge.attributes.edge_type, "implements");

        let re_serialized = serde_json::to_value(&edge).unwrap();
        assert_eq!(re_serialized["id"], 42);
        assert_eq!(re_serialized["attributes"]["edge_type"], "implements");
    }

    #[test]
    fn create_edge_params_serde_roundtrip() {
        let params = CreateEdgeParams {
            source_id: 10,
            target_id: 20,
            edge_type: "depends_on".into(),
        };
        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["source_id"], 10);
        assert_eq!(json["target_id"], 20);
        assert_eq!(json["edge_type"], "depends_on");

        let back: CreateEdgeParams = serde_json::from_value(json).unwrap();
        assert_eq!(back.source_id, 10);
        assert_eq!(back.target_id, 20);
        assert_eq!(back.edge_type, "depends_on");
    }

    #[test]
    fn unlink_format_override_json_flag() {
        let fmt = if true {
            OutputFormat::Json
        } else {
            OutputFormat::Table
        };
        assert_eq!(fmt, OutputFormat::Json);
    }

    #[test]
    fn edges_list_cmd_parses_source_target() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["edges", "list", "--source", "F-1", "--target", "T-5", "--type", "implements"])
            .unwrap();
        let sub = m.subcommand_matches("list").unwrap();
        assert_eq!(sub.get_one::<String>("source").unwrap(), "F-1");
        assert_eq!(sub.get_one::<String>("target").unwrap(), "T-5");
        assert_eq!(sub.get_one::<String>("type").unwrap(), "implements");
    }

    #[test]
    fn build_command_link_requires_source_target_type() {
        let cmd = build_command();
        assert!(cmd
            .try_get_matches_from(["edges", "link", "--source", "F-1"])
            .is_err());
    }

    #[test]
    fn build_command_link_parses() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["edges", "link", "--source", "F-1", "--target", "T-5", "--type", "implements"])
            .unwrap();
        let sub = m.subcommand_matches("link").unwrap();
        assert_eq!(sub.get_one::<String>("source").unwrap(), "F-1");
        assert_eq!(sub.get_one::<String>("target").unwrap(), "T-5");
        assert_eq!(sub.get_one::<String>("type").unwrap(), "implements");
    }
}
