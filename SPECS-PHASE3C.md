=== EXPORT SPECS ===

## F-7: Export Commands

Export project specs in various formats, leveraging the server-side export API.

**Commands:**
- `funcspec export` — Export current project spec (default: markdown to stdout)
- `funcspec export --format md|json|csv|html|pdf|docx` — Choose format
- `funcspec export -o spec.pdf` — Write to file instead of stdout
- `funcspec export --type func` — Export only functional items
- `funcspec export --type tech` — Export only technical items
- `funcspec export --tag v1` — Export only tagged items

**Behavior:**
- Markdown, JSON, CSV: stream to stdout (pipeable)
- HTML: write to file, optionally open in browser with `--open`
- PDF, DOCX: binary formats, require `-o` flag or default to `<project-slug>.<ext>`

**View mode:**
- `funcspec view` — Open the project's shareable HTML view URL in the default browser
- `funcspec view F-377` — Open a specific item in the browser

**Examples:**
```
funcspec export --format json | jq ".items | length"
funcspec export --format md -o SPEC.md
funcspec export --format pdf -o funcspec-cli-v1.pdf --tag v1
```

## T-66: Export command infrastructure with output format handling

Implement ExportCommand struct in src/commands/export.rs with comprehensive export functionality. Fields: format (ExportFormat enum: Markdown, Json, Csv, Html, Pdf, Docx), output_path (Option<PathBuf>), item_type (Option<ItemType> enum: Func, Tech), tag (Option<String>), open_browser (bool for HTML). Include validate() method to enforce business rules: PDF/DOCX require output file or default naming, HTML with --open requires file output. Implement run() method that: 1) fetches project data via client, 2) filters items by type/tag if specified, 3) calls appropriate formatter, 4) handles output (stdout vs file), 5) opens browser for HTML if requested. Add helper methods: default_filename() for binary formats using project slug, should_write_to_file() logic. Include comprehensive error handling for file I/O, format validation, and missing project context.

**Rationale:** Core command logic separate from formatting implementations

## T-67: Streaming export formatters with file output and consistent filtering

Create src/export/formatters.rs module with streaming-capable export system. Define ExportFormatter trait with methods: fn format_stream(&self, project: &Project, items: impl Iterator<Item = &Item>, writer: &mut dyn Write) -> Result<()> and fn format_to_string(&self, project: &Project, items: &[Item]) -> Result<String> for compatibility. Implement ExportConfig struct with fields: date_format, include_fields: HashSet<String>, chunk_size: usize for streaming batches. Define FilterCriteria struct with optional fields: status_filter, date_range, tags, item_types for consistent filtering across formatters. Create ExportStructure template with sections: header (title, created_at, version), metadata (project details, export_config), items (filtered and formatted), footer (summary stats). Implement format-specific exporters: MarkdownFormatter generates clean markdown with hierarchical headers, bullet-point listings, code-fenced metadata blocks, and proper escaping via escape_markdown(). JsonFormatter uses serde_json with streaming serializer, maintains API response structure {project: {...}, items: [...], metadata: {...}}. CsvFormatter implements with csv crate writer, columns [id, type, title, description, status, created_at, tags], handles streaming rows with proper field escaping. HtmlFormatter generates standalone HTML with embedded CSS, responsive navigation, sanitized content via sanitize_html(), and item cross-references using id anchors. Add src/export/writer.rs module with FileWriter struct implementing Write trait for file output to specified paths with error handling. Include filtering module with apply_filter(items: &[Item], criteria: &FilterCriteria) -> Vec<&Item> function used consistently across all formatters. Add helper functions: escape_markdown(), sanitize_html(), format_tags(), format_date_with_config(). Implement comprehensive error handling with ExportError enum covering IoError, SerializationError, FilterError. Include unit tests for each formatter with sample data, streaming tests with large datasets, file output integration tests, and filtering validation tests.

## T-68: Binary format export with external tool integration

Implement BinaryExportService in src/export/binary.rs for PDF and DOCX generation. PDF generation: integrate wkhtmltopdf or similar tool, convert HTML output to PDF with proper styling, headers, and page breaks. Include dependency detection, installation guidance, and fallback error messages. DOCX generation: use external tool or library (docx-rs if available) to convert structured content to Word format with proper headings, lists, and metadata. Implement ExportTool trait with methods: is_available(), install_instructions(), convert(). Handle tool execution with proper error handling, temporary file management, and cleanup. Include configuration for tool paths and options. Add timeout handling and progress indication for long exports. Provide clear error messages when tools are missing with installation instructions.

**Rationale:** Binary formats require external tools and complex integration logic

## T-69: View command for browser integration

Implement ViewCommand struct in src/commands/view.rs for opening project URLs in browser. Fields: item_id (Option<String>) for specific item viewing. Implement run() method that: 1) constructs appropriate URL (project overview or specific item), 2) uses opener crate to launch default browser, 3) handles URL generation with proper encoding. Include URL builder methods: project_url(), item_url() that construct shareable URLs based on project context and item IDs. Add error handling for missing project context, invalid item IDs, and browser launch failures. Include cross-platform browser detection and fallback mechanisms. Provide user feedback on URL opening success/failure. Add --dry-run flag to print URL instead of opening browser for debugging.

**Rationale:** Browser integration is distinct functionality with different error handling

## T-70: Export API client methods with filtering support

Extend FuncSpecClient in src/client.rs with export-specific methods. Add export_project() method that accepts ProjectExportOptions struct with fields: format, item_type_filter, tag_filter, include_metadata. Method should handle API endpoint /projects/{id}/export with query parameters for server-side filtering and format hints. Implement get_project_with_items() method for full project data retrieval with items collection. Add helper methods: filter_items_by_type(), filter_items_by_tag() for client-side filtering when server-side isn't available. Include proper error handling for export API failures, large response handling, and timeout configuration. Add caching mechanisms for repeated exports of same project. Implement response streaming for large exports to handle memory efficiently.

**Rationale:** API integration needs dedicated methods with proper filtering and error handling

## T-71: CLI argument parsing for export and view commands

Extend src/cli.rs with ExportArgs and ViewArgs structs using clap derives. ExportArgs fields: format (ValueEnum for md|json|csv|html|pdf|docx with default markdown), output (Option<PathBuf> for -o flag), item_type (Option<ValueEnum> for --type func|tech), tag (Option<String> for --tag), open (bool flag for --open, HTML only). ViewArgs fields: item_id (Option<String> positional argument). Add proper clap attributes: value names, help texts, conflicts (--open only valid with HTML), validation (output required for binary formats). Include examples in help text and proper error messages for invalid combinations. Add shell completion support for format options and common filenames. Implement From<ExportArgs> for ExportCommand and From<ViewArgs> for ViewCommand conversion methods.

**Rationale:** CLI parsing logic should be centralized and properly validated before command execution
