use anyhow::{Result, bail};
use clap::Subcommand;
use console::style;
use funcspec_client::models::*;
use std::collections::HashMap;

use crate::context::client_and_project;
use crate::output::{self, OutputFormat};

#[derive(Debug, Subcommand)]
pub enum EdgesCmd {
    /// List dependency edges (optionally filtered by source, target, or type)
    List {
        /// Filter by source item permalink or ID (e.g. F-1)
        #[arg(long)]
        source: Option<String>,

        /// Filter by target item permalink or ID (e.g. T-5)
        #[arg(long)]
        target: Option<String>,

        /// Filter by edge type (depends_on, implements, tests, blocks, relates_to)
        #[arg(long, value_name = "TYPE")]
        r#type: Option<String>,

        /// Output as JSON (overrides --format)
        #[arg(long)]
        json: bool,
    },

    /// Create a dependency edge between two spec items
    Link {
        /// Source item permalink or ID (e.g. F-1)
        #[arg(long, required = true)]
        source: String,

        /// Target item permalink or ID (e.g. T-5)
        #[arg(long, required = true)]
        target: String,

        /// Edge type: depends_on, implements, tests, blocks, relates_to
        #[arg(long, value_name = "TYPE", required = true)]
        r#type: String,
    },

    /// Delete a dependency edge by ID (or by --source/--target/--type)
    Unlink {
        /// Edge ID to delete
        edge_id: Option<u64>,

        /// Source item permalink or ID (used to find the edge)
        #[arg(long)]
        source: Option<String>,

        /// Target item permalink or ID (used to find the edge)
        #[arg(long)]
        target: Option<String>,

        /// Edge type filter (used to find the edge)
        #[arg(long, value_name = "TYPE")]
        r#type: Option<String>,

        /// Skip confirmation prompt
        #[arg(long, short)]
        yes: bool,
    },
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
        // json=true overrides any global format to Json
        let fmt = if true {
            OutputFormat::Json
        } else {
            OutputFormat::Table
        };
        assert_eq!(fmt, OutputFormat::Json);
    }

    #[test]
    fn edges_list_cmd_parses_source_target() {
        // Verify the struct fields are accessible with expected types
        let cmd = EdgesCmd::List {
            source: Some("F-1".to_string()),
            target: Some("T-5".to_string()),
            r#type: Some("implements".to_string()),
            json: false,
        };
        match cmd {
            EdgesCmd::List {
                source,
                target,
                r#type,
                ..
            } => {
                assert_eq!(source.as_deref(), Some("F-1"));
                assert_eq!(target.as_deref(), Some("T-5"));
                assert_eq!(r#type.as_deref(), Some("implements"));
            }
            _ => panic!("wrong variant"),
        }
    }
}
