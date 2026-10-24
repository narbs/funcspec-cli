use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use colored::Colorize;
use funcspec_client::models::{ImportItem, ImportPreview};

use crate::context::client_and_project;

pub struct ImportArgs {
    pub file: PathBuf,
    pub version: Option<String>,
    pub dry_run: bool,
    pub json: bool,
}

pub fn build_command() -> clap::Command {
    clap::Command::new("import")
        .about("Import spec items from a CSV or JSON file")
        .long_about(
            "Import spec items from a CSV or JSON file.\n\n\
             Sends the file to the server for preview (validation + parse), \
             then commits the items. Use --dry-run to preview only.\n\n\
             CSV columns: title, description, type_of, parent_title, position\n\
             JSON format: array of item objects with the same fields.\n\n\
             With --version the created items are assigned to that release version \
             as a post-commit step (the server commit endpoint does not take a version).",
        )
        .arg(
            clap::Arg::new("file")
                .required(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help("CSV or JSON file to import"),
        )
        .arg(
            clap::Arg::new("version")
                .long("version")
                .value_name("SEMVER")
                .help(
                    "Assign created items to this release version (semver or id). \
                     NOTE: version assignment is a post-commit step.",
                ),
        )
        .arg(
            clap::Arg::new("dry_run")
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help("Preview only — validate and print counts, do not commit"),
        )
        .arg(
            clap::Arg::new("json")
                .long("json")
                .action(clap::ArgAction::SetTrue)
                .help("Machine-readable JSON output"),
        )
}

pub fn from_arg_matches(matches: &clap::ArgMatches) -> ImportArgs {
    ImportArgs {
        file: matches.get_one::<PathBuf>("file").cloned().unwrap(),
        version: matches.get_one::<String>("version").cloned(),
        dry_run: matches.get_flag("dry_run"),
        json: matches.get_flag("json"),
    }
}

fn print_preview(preview: &ImportPreview) {
    let attrs = &preview.attributes;
    println!(
        "Preview: {} items ({} functional, {} technical)",
        attrs.total, attrs.functional_count, attrs.technical_count
    );
    if !attrs.duplicate_titles.is_empty() {
        println!(
            "{} duplicate title(s): {}",
            "Warning:".yellow().bold(),
            attrs.duplicate_titles.join(", ")
        );
    }
    for w in &attrs.warnings {
        println!("{} {}", "Warning:".yellow(), w);
    }
}

pub async fn run(args: ImportArgs) -> Result<()> {
    let raw = std::fs::read_to_string(&args.file)
        .with_context(|| format!("Failed to read {}", args.file.display()))?;

    let (client, project_id) = client_and_project().await?;

    // Step 1: preview — validates file, returns parsed items
    let preview = client.preview_import(project_id, &raw).await?;

    if args.json {
        let preview_json = serde_json::json!({
            "preview": {
                "format": preview.attributes.format,
                "total": preview.attributes.total,
                "functional_count": preview.attributes.functional_count,
                "technical_count": preview.attributes.technical_count,
                "duplicate_titles": preview.attributes.duplicate_titles,
                "warnings": preview.attributes.warnings,
            }
        });
        println!("{}", serde_json::to_string_pretty(&preview_json)?);
    } else {
        print_preview(&preview);
    }

    if args.dry_run {
        eprintln!("(dry-run: not committing)");
        return Ok(());
    }

    // Step 2: commit — use items returned by preview (already validated + normalised)
    let items: Vec<ImportItem> = preview.attributes.items.clone();
    if items.is_empty() {
        bail!("No items to import (preview returned 0 items)");
    }

    // Log that version is a post-commit concern
    if args.version.is_some() {
        eprintln!(
            "Note: version assignment is a post-commit step — \
             the server import/commit endpoint does not accept a release version directly."
        );
    }

    let result = client.commit_import(project_id, &items).await?;

    let func_count = result
        .attributes
        .items
        .iter()
        .filter(|i| i.type_of == "functional")
        .count();
    let tech_count = result
        .attributes
        .items
        .iter()
        .filter(|i| i.type_of == "technical")
        .count();

    if args.json {
        let result_json = serde_json::json!({
            "created": result.attributes.created,
            "functional_count": func_count,
            "technical_count": tech_count,
            "items": result.attributes.items.iter().map(|i| serde_json::json!({
                "id": i.id,
                "permalink": i.permalink,
                "title": i.title,
                "type_of": i.type_of,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&result_json)?);
    } else {
        println!(
            "Created {} items ({} functional, {} technical)",
            result.attributes.created, func_count, tech_count
        );
        for item in &result.attributes.items {
            let label = if item.type_of == "functional" {
                "F".cyan().to_string()
            } else {
                "T".yellow().to_string()
            };
            println!("  {} {} — {}", label, item.permalink, item.title);
        }
    }

    // Step 3: version assignment (post-commit step; server commit takes no version)
    if let Some(ref ver) = args.version {
        let permalinks: Vec<String> = result
            .attributes
            .items
            .iter()
            .map(|i| i.permalink.clone())
            .collect();

        client
            .add_items_to_version(project_id, ver, &permalinks)
            .await
            .with_context(|| format!("Failed to assign items to release version '{ver}'"))?;

        if args.json {
            let assign_json = serde_json::json!({
                "version_assigned": ver,
                "assigned_count": permalinks.len(),
            });
            eprintln!("{}", serde_json::to_string_pretty(&assign_json)?);
        } else {
            println!(
                "Assigned {} items to release version {}",
                permalinks.len(),
                ver.bold()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_command_parses_dry_run() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["import", "items.csv", "--dry-run"])
            .unwrap();
        let args = from_arg_matches(&m);
        assert!(args.dry_run);
        assert_eq!(args.file, PathBuf::from("items.csv"));
        assert!(!args.json);
        assert!(args.version.is_none());
    }

    #[test]
    fn build_command_parses_version_flag() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["import", "items.csv", "--version", "1.0.0"])
            .unwrap();
        let args = from_arg_matches(&m);
        assert_eq!(args.version.as_deref(), Some("1.0.0"));
        assert!(!args.dry_run);
    }

    #[test]
    fn build_command_parses_json_flag() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["import", "items.csv", "--json"])
            .unwrap();
        let args = from_arg_matches(&m);
        assert!(args.json);
    }

    #[test]
    fn build_command_requires_file() {
        let cmd = build_command();
        let result = cmd.try_get_matches_from(["import"]);
        assert!(result.is_err());
    }
}
