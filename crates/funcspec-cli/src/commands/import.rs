use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use colored::Colorize;
use funcspec_client::models::{
    DocumentImportItem, ImportItem, ImportPreview, ImportResultItem, ImportSession,
    ImportSessionStatus,
};
use funcspec_client::{Error as ClientError, FuncspecClient};
use indicatif::{ProgressBar, ProgressStyle};

use crate::context::{client_and_project, client_and_project_audit};

pub struct ImportArgs {
    pub file: PathBuf,
    pub version: Option<String>,
    pub enrich: bool,
    pub dry_run: bool,
    pub json: bool,
}

/// Kind of raw document handled by the server-side extraction pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentKind {
    Pdf,
    Docx,
    Markdown,
    Text,
}

impl DocumentKind {
    fn mime_type(self) -> &'static str {
        match self {
            DocumentKind::Pdf => "application/pdf",
            DocumentKind::Docx => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            }
            DocumentKind::Markdown => "text/markdown",
            DocumentKind::Text => "text/plain",
        }
    }
}

/// Which import path a file takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFormat {
    /// CSV / JSON — the existing preview + commit path.
    Structured,
    /// Raw document — upload + LLM extraction session.
    Document(DocumentKind),
}

const PDF_MAGIC: &[u8] = b"%PDF";
const ZIP_MAGIC: &[u8] = b"PK\x03\x04";

/// Decide the import path from the file extension and leading magic bytes.
pub fn detect_format(path: &Path, head: &[u8]) -> Result<ImportFormat> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let is_pdf = head.starts_with(PDF_MAGIC);
    let is_zip = head.starts_with(ZIP_MAGIC);

    match ext.as_str() {
        "csv" | "json" => Ok(ImportFormat::Structured),
        "pdf" if is_pdf => Ok(ImportFormat::Document(DocumentKind::Pdf)),
        "pdf" => bail!(
            "{} has a .pdf extension but is not a PDF (missing %PDF header)",
            path.display()
        ),
        "docx" if is_zip => Ok(ImportFormat::Document(DocumentKind::Docx)),
        "docx" => bail!(
            "{} has a .docx extension but is not a Word document (missing ZIP header)",
            path.display()
        ),
        _ if is_pdf => Ok(ImportFormat::Document(DocumentKind::Pdf)),
        "md" | "markdown" => Ok(ImportFormat::Document(DocumentKind::Markdown)),
        "txt" | "text" => Ok(ImportFormat::Document(DocumentKind::Text)),
        _ => bail!(
            "Unsupported import file '{}'. Supported: .csv, .json (structured) \
             or .pdf, .docx, .md, .txt (documents)",
            path.display()
        ),
    }
}

pub fn build_command() -> clap::Command {
    clap::Command::new("import")
        .about("Import spec items from a CSV/JSON file or a document (PDF, Word, Markdown, text)")
        .long_about(
            "Import spec items from a structured file (CSV or JSON) or from a raw \
             document (PDF, Word .docx, Markdown, plain text).\n\n\
             Structured (.csv, .json): the file is sent to the server for preview \
             (validation + parse), then the items are committed.\n\
             CSV columns: title, description, type_of, parent_title, position\n\
             JSON format: array of item objects with the same fields.\n\n\
             Documents (.pdf, .docx, .md, .txt): the file is uploaded (10MB max), \
             the server's LLM extracts candidate spec items in the background \
             (this can take a few minutes), the candidates are previewed and then \
             committed. --enrich runs an optional second pass that refines the \
             candidates. LLM cost is reported; the server caps it at $1.00 per import.\n\n\
             Use --dry-run to preview only (for documents, extraction still runs \
             and is billed, but nothing is committed).\n\n\
             With --version the created items are assigned to that release version \
             as a post-commit step (the server commit endpoint does not take a version).",
        )
        .arg(
            clap::Arg::new("file")
                .required(true)
                .value_parser(clap::value_parser!(PathBuf))
                .help("CSV/JSON file or document (.pdf, .docx, .md, .txt) to import"),
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
            clap::Arg::new("enrich")
                .long("enrich")
                .action(clap::ArgAction::SetTrue)
                .help("Documents only: run the optional second LLM pass to refine candidates"),
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
        enrich: matches.get_flag("enrich"),
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
    // Read as bytes: documents (PDF/DOCX) are binary and fail UTF-8 decoding.
    let bytes = std::fs::read(&args.file)
        .with_context(|| format!("Failed to read {}", args.file.display()))?;

    match detect_format(&args.file, &bytes)? {
        ImportFormat::Structured => {
            let raw = String::from_utf8(bytes)
                .with_context(|| format!("{} is not valid UTF-8", args.file.display()))?;
            if args.enrich {
                eprintln!("Note: --enrich only applies to document imports; ignoring.");
            }
            run_structured(args, raw).await
        }
        ImportFormat::Document(kind) => run_document(args, kind, bytes).await,
    }
}

async fn run_structured(args: ImportArgs, raw: String) -> Result<()> {
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
        print_created(&result.attributes.items);
    }

    // Step 3: version assignment (post-commit step; server commit takes no version)
    if let Some(ref ver) = args.version {
        assign_version(
            &client,
            project_id,
            ver,
            &result.attributes.items,
            args.json,
        )
        .await?;
    }

    Ok(())
}

/// Post-commit step: assign created items to a release version.
async fn assign_version(
    client: &FuncspecClient,
    project_id: u64,
    ver: &str,
    items: &[ImportResultItem],
    json: bool,
) -> Result<()> {
    let permalinks: Vec<String> = items.iter().map(|i| i.permalink.clone()).collect();

    client
        .add_items_to_version(project_id, ver, &permalinks)
        .await
        .with_context(|| format!("Failed to assign items to release version '{ver}'"))?;

    if json {
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
    Ok(())
}

fn print_created(items: &[ImportResultItem]) {
    for item in items {
        let label = if item.type_of == "functional" {
            "F".cyan().to_string()
        } else {
            "T".yellow().to_string()
        };
        println!("  {} {} — {}", label, item.permalink, item.title);
    }
}

// ---------------------------------------------------------------------------
// Document import (T-528): upload -> extract [-> enrich] -> poll -> commit
// ---------------------------------------------------------------------------

/// Upper bound on waiting for one extraction pass (the server-side LLM job).
const EXTRACTION_WAIT: Duration = Duration::from_secs(15 * 60);
const POLL_INTERVAL: Duration = Duration::from_secs(2);

fn format_cents(cents: u64) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

/// Map a document-import server error code to a clear CLI message.
pub fn describe_import_error(code: &str, message: &str) -> String {
    let hint = match code {
        "unsupported_format" => "Unsupported document format. Supported: .pdf, .docx, .md, .txt.",
        "file_too_large" => "File too large — the server accepts documents up to 10MB.",
        "password_protected" => {
            "The document is password-protected; remove the password and retry."
        }
        "corrupt_file" => "The document could not be read (corrupt or malformed file).",
        "extraction_timeout" => {
            "Extraction timed out on the server. Try a smaller document or retry."
        }
        "cost_limit" => "The $1.00 per-import LLM cost cap was reached.",
        "llm_not_configured" => {
            "No LLM provider is configured for this organisation. Configure one in organisation settings."
        }
        "extraction_failed" => "The server failed to extract spec items from the document.",
        "forbidden" => {
            "Permission denied — document import needs an API key tied to a user with editor access \
             to this project."
        }
        "no_content" => "The document contains no extractable text.",
        "no_items" => "There are no extracted items to import.",
        "invalid_state" => "The import session is not in a state that allows this action.",
        "import_failed" => "The server rejected the import.",
        _ => "",
    };
    match (hint.is_empty(), message.is_empty()) {
        (true, true) => format!("Document import failed ({code})"),
        (true, false) => format!("{message} ({code})"),
        (false, true) => format!("{hint} ({code})"),
        (false, false) => format!("{hint} Server said: {message} ({code})"),
    }
}

fn import_error(e: ClientError) -> anyhow::Error {
    match e {
        ClientError::Api { code, message, .. } => {
            anyhow::anyhow!(describe_import_error(&code, &message))
        }
        other => anyhow::anyhow!(other.user_message()),
    }
}

fn make_spinner(msg: &str, hidden: bool) -> ProgressBar {
    if hidden {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Start an extract/enrich stage. A 409 means a run is already in progress
/// for this session — attach to it by polling rather than failing.
async fn start_stage(
    client: &FuncspecClient,
    project_id: u64,
    session_id: u64,
    enrich: bool,
) -> Result<()> {
    let started = if enrich {
        client.start_enrich(project_id, session_id).await
    } else {
        client.start_extract(project_id, session_id).await
    };
    match started {
        Ok(_) => Ok(()),
        Err(ClientError::Api { status: 409, .. }) => {
            let session = client
                .poll_import_session(project_id, session_id)
                .await
                .map_err(import_error)?;
            if session.status == ImportSessionStatus::Extracting {
                eprintln!(
                    "An extraction run is already in progress for this session; waiting for it."
                );
                Ok(())
            } else {
                bail!(
                    "Cannot start {}: import session {session_id} is {}",
                    if enrich { "enrichment" } else { "extraction" },
                    session.status
                )
            }
        }
        Err(e) => Err(import_error(e)),
    }
}

/// Poll the session until the background run settles (reviewing / failed / …).
async fn wait_for_session(
    client: &FuncspecClient,
    project_id: u64,
    session_id: u64,
    label: &str,
    quiet: bool,
) -> Result<ImportSession> {
    let pb = make_spinner(&format!("{label} (session {session_id})…"), quiet);
    let start = Instant::now();
    let poll = async {
        loop {
            let session = client.poll_import_session(project_id, session_id).await?;
            if session.status.is_settled() {
                return Ok::<_, ClientError>(Some(session));
            }
            pb.set_message(format!(
                "{label} (session {session_id}) — status: {}, {}s elapsed",
                session.status,
                start.elapsed().as_secs()
            ));
            if start.elapsed() >= EXTRACTION_WAIT {
                return Ok(None);
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    };

    let outcome = tokio::select! {
        r = poll => r,
        _ = tokio::signal::ctrl_c() => {
            pb.finish_and_clear();
            bail!(
                "Interrupted. The server keeps working on import session {session_id}; \
                 it was not committed."
            );
        }
    };
    pb.finish_and_clear();

    match outcome.map_err(import_error)? {
        Some(session) => Ok(session),
        None => bail!(
            "Gave up waiting after {}s; import session {session_id} is still extracting on the server.",
            EXTRACTION_WAIT.as_secs()
        ),
    }
}

/// Fail with a clear message unless the session is ready for review/commit.
fn ensure_reviewing(session: &ImportSession) -> Result<()> {
    match session.status {
        ImportSessionStatus::Reviewing => Ok(()),
        ImportSessionStatus::Failed => {
            let code = session.error_code.as_deref().unwrap_or("extraction_failed");
            let message = session.error_message.as_deref().unwrap_or("");
            bail!(
                "{} [cost so far: {}]",
                describe_import_error(code, message),
                format_cents(session.cost_cents)
            )
        }
        other => bail!(
            "Import session {} ended in unexpected state '{other}'",
            session.session_id
        ),
    }
}

fn print_candidates(items: &[DocumentImportItem]) {
    for item in items {
        let label = match item.type_of.as_str() {
            "functional" => "F".cyan().to_string(),
            "technical" => "T".yellow().to_string(),
            _ => "?".dimmed().to_string(),
        };
        let confidence = match &item.confidence {
            Some(serde_json::Value::Number(n)) => format!(" ({n})"),
            Some(serde_json::Value::String(s)) => format!(" ({s})"),
            _ => String::new(),
        };
        let skipped = if item.included == Some(false) {
            " [excluded]".dimmed().to_string()
        } else {
            String::new()
        };
        println!(
            "  {} {}{}{}",
            label,
            item.title,
            confidence.dimmed(),
            skipped
        );
    }
}

async fn run_document(args: ImportArgs, kind: DocumentKind, bytes: Vec<u8>) -> Result<()> {
    // Long timeout: upload does synchronous text extraction server-side.
    let (client, project_id) = client_and_project_audit().await?;
    let filename = args
        .file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document")
        .to_string();

    // Step 1: upload
    let pb = make_spinner(&format!("Uploading {filename}…"), args.json);
    let upload = client
        .upload_document(project_id, &filename, kind.mime_type(), bytes)
        .await;
    pb.finish_and_clear();
    let upload = upload.map_err(import_error)?;
    let session_id = upload.session_id;
    if !args.json {
        println!(
            "Uploaded {} — {} section(s) found (import session {})",
            filename.bold(),
            upload.sections_count,
            session_id
        );
        for w in &upload.warnings {
            println!("{} {}", "Warning:".yellow(), w);
        }
    }

    // Step 2: pass 1 (outline extraction)
    start_stage(&client, project_id, session_id, false).await?;
    let mut session = wait_for_session(
        &client,
        project_id,
        session_id,
        "Extracting spec items",
        args.json,
    )
    .await?;
    ensure_reviewing(&session)?;

    // Step 3: optional pass 2 (enrich)
    if args.enrich {
        start_stage(&client, project_id, session_id, true).await?;
        session = wait_for_session(
            &client,
            project_id,
            session_id,
            "Enriching spec items",
            args.json,
        )
        .await?;
        ensure_reviewing(&session)?;
    }

    // Step 4: preview candidates
    let func_count = session
        .items
        .iter()
        .filter(|i| i.type_of == "functional")
        .count();
    let tech_count = session
        .items
        .iter()
        .filter(|i| i.type_of == "technical")
        .count();
    if args.json {
        let preview_json = serde_json::json!({
            "preview": {
                "format": "document",
                "session_id": session_id,
                "sections_count": upload.sections_count,
                "total": session.items.len(),
                "functional_count": func_count,
                "technical_count": tech_count,
                "cost_cents": session.cost_cents,
                "cost_limit_cents": session.cost_limit_cents,
                "warnings": session.warnings,
                "items": session.items,
            }
        });
        println!("{}", serde_json::to_string_pretty(&preview_json)?);
    } else {
        println!(
            "Preview: {} candidate items ({} functional, {} technical) — LLM cost {}",
            session.items.len(),
            func_count,
            tech_count,
            format_cents(session.cost_cents)
        );
        for w in &session.warnings {
            println!("{} {}", "Warning:".yellow(), w);
        }
        print_candidates(&session.items);
    }

    if args.dry_run {
        eprintln!("(dry-run: not committing; import session {session_id} left in review)");
        return Ok(());
    }

    if session.items.is_empty() {
        bail!("No items to import (extraction returned 0 candidates)");
    }

    if args.version.is_some() {
        eprintln!(
            "Note: version assignment is a post-commit step — \
             the server import commit endpoint does not accept a release version directly."
        );
    }

    // Step 5: commit the session's extracted items
    let result = client
        .commit_import_session(project_id, session_id, None)
        .await
        .map_err(import_error)?;

    let func_created = result
        .items
        .iter()
        .filter(|i| i.type_of == "functional")
        .count();
    let tech_created = result
        .items
        .iter()
        .filter(|i| i.type_of == "technical")
        .count();
    if args.json {
        let result_json = serde_json::json!({
            "session_id": session_id,
            "created": result.created,
            "functional_count": func_created,
            "technical_count": tech_created,
            "cost_cents": session.cost_cents,
            "items": result.items.iter().map(|i| serde_json::json!({
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
            result.created, func_created, tech_created
        );
        print_created(&result.items);
    }

    // Step 6: version assignment (post-commit)
    if let Some(ref ver) = args.version {
        assign_version(&client, project_id, ver, &result.items, args.json).await?;
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
    fn build_command_parses_enrich_flag() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["import", "spec.pdf", "--enrich", "--dry-run", "--json"])
            .unwrap();
        let args = from_arg_matches(&m);
        assert!(args.enrich);
        assert!(args.dry_run);
        assert!(args.json);
        assert_eq!(args.file, PathBuf::from("spec.pdf"));
    }

    #[test]
    fn enrich_defaults_off() {
        let m = build_command()
            .try_get_matches_from(["import", "spec.md"])
            .unwrap();
        assert!(!from_arg_matches(&m).enrich);
    }

    #[test]
    fn detect_structured_formats() {
        for name in ["items.csv", "items.json", "ITEMS.CSV"] {
            assert_eq!(
                detect_format(Path::new(name), b"title,description").unwrap(),
                ImportFormat::Structured,
                "{name}"
            );
        }
    }

    #[test]
    fn detect_pdf_by_extension_and_magic() {
        assert_eq!(
            detect_format(Path::new("spec.pdf"), b"%PDF-1.7\n...").unwrap(),
            ImportFormat::Document(DocumentKind::Pdf)
        );
        // Magic bytes win for an unknown extension.
        assert_eq!(
            detect_format(Path::new("download.bin"), b"%PDF-1.4").unwrap(),
            ImportFormat::Document(DocumentKind::Pdf)
        );
        // Extension without magic is rejected.
        assert!(detect_format(Path::new("fake.pdf"), b"hello").is_err());
    }

    #[test]
    fn detect_docx_by_extension_and_magic() {
        assert_eq!(
            detect_format(Path::new("Spec.DOCX"), b"PK\x03\x04rest").unwrap(),
            ImportFormat::Document(DocumentKind::Docx)
        );
        assert!(detect_format(Path::new("broken.docx"), b"not a zip").is_err());
        // A bare zip is not a document we know how to import.
        assert!(detect_format(Path::new("archive.zip"), b"PK\x03\x04").is_err());
    }

    #[test]
    fn detect_text_documents() {
        assert_eq!(
            detect_format(Path::new("notes.md"), b"# Title").unwrap(),
            ImportFormat::Document(DocumentKind::Markdown)
        );
        assert_eq!(
            detect_format(Path::new("notes.txt"), b"plain").unwrap(),
            ImportFormat::Document(DocumentKind::Text)
        );
    }

    #[test]
    fn detect_rejects_unknown() {
        assert!(detect_format(Path::new("image.png"), b"\x89PNG").is_err());
        assert!(detect_format(Path::new("noext"), b"").is_err());
    }

    #[test]
    fn describe_import_error_maps_codes() {
        assert!(describe_import_error("file_too_large", "").contains("10MB"));
        assert!(describe_import_error("cost_limit", "").contains("$1.00"));
        assert!(describe_import_error("llm_not_configured", "x").contains("LLM provider"));
        let unknown = describe_import_error("weird", "boom");
        assert!(unknown.contains("boom") && unknown.contains("weird"));
    }

    #[test]
    fn format_cents_renders_dollars() {
        assert_eq!(format_cents(0), "$0.00");
        assert_eq!(format_cents(7), "$0.07");
        assert_eq!(format_cents(123), "$1.23");
    }

    #[test]
    fn build_command_requires_file() {
        let cmd = build_command();
        let result = cmd.try_get_matches_from(["import"]);
        assert!(result.is_err());
    }
}
