use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Args, ValueEnum};
use funcspec_client::ExportData;

use crate::context::client_and_config;

/// Export format options.
#[derive(Debug, Clone, PartialEq, ValueEnum)]
pub enum ExportFormat {
    /// Markdown (default)
    #[value(name = "md")]
    Md,
    /// JSON
    Json,
    /// CSV
    Csv,
    /// Self-contained HTML
    Html,
    /// PDF (binary)
    Pdf,
    /// DOCX (binary)
    Docx,
}

impl ExportFormat {
    fn api_name(&self) -> &'static str {
        match self {
            ExportFormat::Md => "markdown",
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Html => "html",
            ExportFormat::Pdf => "pdf",
            ExportFormat::Docx => "docx",
        }
    }

    fn is_binary(&self) -> bool {
        matches!(self, ExportFormat::Pdf | ExportFormat::Docx)
    }

    fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Md => "md",
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Html => "html",
            ExportFormat::Pdf => "pdf",
            ExportFormat::Docx => "docx",
        }
    }
}

/// Arguments for `funcspec export`.
#[derive(Debug, Args)]
#[command(
    about = "Export project spec in various formats",
    long_about = "Export the current project spec as markdown, JSON, CSV, HTML, PDF, or DOCX.\n\
                  Text formats (md, json, csv) stream to stdout unless -o is given.\n\
                  Binary formats (pdf, docx) require -o or default to <slug>.<ext>."
)]
pub struct ExportArgs {
    /// Output format
    #[arg(long, short = 'f', value_enum, default_value = "md")]
    pub format: ExportFormat,

    /// Write output to this file instead of stdout
    #[arg(long, short = 'o', value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Filter by item type
    #[arg(long, value_name = "TYPE", help = "Filter by type: func or tech")]
    pub r#type: Option<String>,

    /// Filter by tag
    #[arg(long)]
    pub tag: Option<String>,

    /// Open in browser after writing (HTML only)
    #[arg(long)]
    pub open: bool,
}

pub async fn run(args: ExportArgs) -> Result<()> {
    if args.open && args.format != ExportFormat::Html {
        bail!("--open is only valid with --format html");
    }

    let (client, config) = client_and_config()?;
    let profile = config
        .active_profile()
        .context("Not logged in. Run `funcspec auth login`.")?;
    let project_slug = profile
        .default_project
        .as_deref()
        .context("No default project set. Run `funcspec projects set-default <slug>`.")?;
    let project = client
        .get_project(project_slug)
        .await
        .with_context(|| format!("Project '{}' not found", project_slug))?;
    let project_id = project.id;
    let slug = &project.attributes.slug;

    let item_type = args.r#type.as_deref().map(|t| match t {
        "func" | "functional" => "functional",
        "tech" | "technical" => "technical",
        other => other,
    });

    let data = client
        .export_project(project_id, args.format.api_name(), item_type, args.tag.as_deref())
        .await?;

    // Resolve output path: explicit > default for binary > none (stdout)
    let output_path: Option<PathBuf> = if let Some(p) = args.output {
        Some(p)
    } else if args.format.is_binary() {
        Some(PathBuf::from(format!("{}.{}", slug, args.format.extension())))
    } else {
        None
    };

    match data {
        ExportData::Text(text) => {
            if let Some(path) = output_path {
                std::fs::write(&path, &text)
                    .with_context(|| format!("Failed to write {}", path.display()))?;
                eprintln!("Exported to {}", path.display());
                if args.open {
                    open::that(&path)
                        .with_context(|| format!("Failed to open {}", path.display()))?;
                }
            } else {
                if args.open {
                    bail!("--open requires -o <path> to specify the output file");
                }
                print!("{text}");
            }
        }
        ExportData::Binary(bytes) => {
            // output_path is always Some for binary (set above)
            let path = output_path.unwrap();
            std::fs::write(&path, &bytes)
                .with_context(|| format!("Failed to write {}", path.display()))?;
            eprintln!("Exported to {}", path.display());
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_api_names() {
        assert_eq!(ExportFormat::Md.api_name(), "markdown");
        assert_eq!(ExportFormat::Json.api_name(), "json");
        assert_eq!(ExportFormat::Csv.api_name(), "csv");
        assert_eq!(ExportFormat::Html.api_name(), "html");
        assert_eq!(ExportFormat::Pdf.api_name(), "pdf");
        assert_eq!(ExportFormat::Docx.api_name(), "docx");
    }

    #[test]
    fn binary_formats() {
        assert!(ExportFormat::Pdf.is_binary());
        assert!(ExportFormat::Docx.is_binary());
        assert!(!ExportFormat::Md.is_binary());
        assert!(!ExportFormat::Json.is_binary());
        assert!(!ExportFormat::Csv.is_binary());
        assert!(!ExportFormat::Html.is_binary());
    }

    #[test]
    fn extensions() {
        assert_eq!(ExportFormat::Md.extension(), "md");
        assert_eq!(ExportFormat::Json.extension(), "json");
        assert_eq!(ExportFormat::Csv.extension(), "csv");
        assert_eq!(ExportFormat::Html.extension(), "html");
        assert_eq!(ExportFormat::Pdf.extension(), "pdf");
        assert_eq!(ExportFormat::Docx.extension(), "docx");
    }

    #[test]
    fn default_binary_filename_uses_slug() {
        let slug = "my-project";
        let pdf_path = PathBuf::from(format!("{}.{}", slug, ExportFormat::Pdf.extension()));
        assert_eq!(pdf_path, PathBuf::from("my-project.pdf"));

        let docx_path = PathBuf::from(format!("{}.{}", slug, ExportFormat::Docx.extension()));
        assert_eq!(docx_path, PathBuf::from("my-project.docx"));
    }

    #[test]
    fn type_mapping_func() {
        let mapped = Some("func").map(|t| match t {
            "func" | "functional" => "functional",
            "tech" | "technical" => "technical",
            other => other,
        });
        assert_eq!(mapped, Some("functional"));
    }

    #[test]
    fn type_mapping_tech() {
        let mapped = Some("tech").map(|t| match t {
            "func" | "functional" => "functional",
            "tech" | "technical" => "technical",
            other => other,
        });
        assert_eq!(mapped, Some("technical"));
    }

    #[test]
    fn type_mapping_full_names() {
        let mapped_f = Some("functional").map(|t| match t {
            "func" | "functional" => "functional",
            "tech" | "technical" => "technical",
            other => other,
        });
        assert_eq!(mapped_f, Some("functional"));

        let mapped_t = Some("technical").map(|t| match t {
            "func" | "functional" => "functional",
            "tech" | "technical" => "technical",
            other => other,
        });
        assert_eq!(mapped_t, Some("technical"));
    }

    #[test]
    fn file_write_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("spec.md");
        std::fs::write(&path, "# Hello").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "# Hello");
    }

    #[test]
    fn binary_write_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("spec.pdf");
        let bytes = b"%PDF-1.4 content".to_vec();
        std::fs::write(&path, &bytes).unwrap();
        let read_back = std::fs::read(&path).unwrap();
        assert_eq!(read_back, bytes);
    }
}
