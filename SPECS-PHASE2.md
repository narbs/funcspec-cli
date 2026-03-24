=== PHASE 2 SPECS ===

## F-6: Output Formatting & Display

Consistent, flexible output formatting across all commands.

**Format flags (global):**
- Default: human-readable table with aligned columns, color-coded status badges
- `--json`: Machine-readable JSON output (for `jq` piping)
- `--quiet` / `-q`: Minimal output — permalinks or IDs only, one per line
- `--bare`: Tab-separated values without table borders, headers, or decoration. Columns match the table view for each command. Ideal for grep, awk, cut, and Unix text processing pipelines.
- `--csv`: CSV output with headers
- `--no-color`: Disable ANSI color codes (auto-detected when stdout is not a TTY)
- `--wide` / `-w`: Show all columns (default hides some for terminal width)

**Format flag precedence:** `--json` > `--quiet` > `--bare` > `--csv` > default table. Only one format flag should be active; if multiple are passed, highest precedence wins.

**Color coding:**
- Implementation status: 🟢 implemented, 🟡 in_progress, ⚪ not_started
- Review verdict: 🟢 pass, 🟡 needs_refinement, 🔴 major_gaps
- Item type: blue for functional, purple for technical
- Color is stripped automatically in `--bare`, `--csv`, `--json`, and `--quiet` modes

**Table output:**
- Auto-truncate long fields to terminal width
- Respect `COLUMNS` env var
- Header row with field names
- Item count footer: "Showing 25 of 142 items"

**Detail view (show commands):**
- Markdown rendering in terminal (bold, lists, code blocks)
- Section headers for description, review, related items
- Clickable URLs where terminal supports them (OSC 8)

**Diff display (improve command):**
- Side-by-side or unified diff format
- `--diff-format unified|side-by-side` (default: unified)
- Color-coded additions/deletions

**Pager:**
- Pipe long output through `$PAGER` (default: `less -R`) when stdout is TTY
- `--no-pager` to disable

## T-28: OutputFormat enum and FormatFlags struct with precedence logic

Create src/output/format.rs with OutputFormat enum (Table, Json, Quiet, Bare, Csv) and FormatFlags struct. FormatFlags fields: json (bool), quiet (bool), bare (bool), csv (bool), no_color (bool), wide (bool), no_pager (bool), diff_format (DiffFormat enum). Implement precedence logic in OutputFormat::from_flags() method: json > quiet > bare > csv > table (default). Add validation to ensure only one primary format flag is active. Include serde serialization for FormatFlags. Add unit tests for precedence scenarios and edge cases.

**Rationale:** Core data structures for output formatting need to be established before any rendering logic

## T-29: Terminal detection and color management utilities

Implement src/output/terminal.rs with terminal capability detection. Functions: is_tty() -> bool (check if stdout is TTY), supports_color() -> bool (check TERM env var and TTY status), get_terminal_width() -> usize (respect COLUMNS env var, fallback to terminal detection). ColorMode enum (Always, Auto, Never) with logic to strip ANSI codes based on format flags and terminal detection. Include atty and termsize crate dependencies. Handle edge cases like CI environments, redirected output, and unsupported terminals.

**Rationale:** Terminal detection is needed by multiple output formatters and should be centralized

## T-30: Table formatter with auto-truncation and column management

Create src/output/table.rs with TableFormatter struct. Methods: format_items<T: TableRow>() -> String where TableRow trait defines get_columns() -> Vec<Column>. Column struct: name (String), value (String), width (Option<usize>), truncatable (bool). Implement auto-truncation logic based on terminal width, preserve important columns when truncating. Add wide mode support to show all columns. Include color-coded status badges using Unicode symbols (🟢🟡⚪🔴). Format table with aligned columns, borders, and headers. Add item count footer. Handle empty result sets gracefully.

**Rationale:** Table formatting is the default output mode and requires complex width calculation logic

## T-31: JSON, CSV, and minimal format renderers

Implement src/output/renderers.rs with JsonRenderer, CsvRenderer, QuietRenderer, and BareRenderer structs. JsonRenderer: format_items<T: Serialize>() -> String with proper indentation. CsvRenderer: format_items<T: CsvRow>() -> String with headers and proper escaping. QuietRenderer: extract and format only permalink/ID fields, one per line. BareRenderer: tab-separated values without borders or decoration, matching table column order. All renderers implement Renderer trait with format_items() method. Handle empty collections and serialization errors. Strip color codes automatically in non-table formats.

**Rationale:** Different output formats require specialized rendering logic that should be modular

## T-32: Markdown renderer for detail views with terminal formatting

Create src/output/markdown.rs with MarkdownRenderer struct. Implement terminal markdown rendering: **bold** text using ANSI codes, bullet lists with proper indentation, `code blocks` with background highlighting, section headers with underlines. Methods: render_description(), render_sections(), render_links(). Support clickable URLs using OSC 8 escape sequences where supported. Handle code blocks, nested lists, and inline formatting. Graceful degradation for terminals without advanced support. Include crossterm dependency for ANSI styling.

**Rationale:** Detail views need specialized markdown rendering that differs from table output

## T-33: Diff formatter with unified and side-by-side display modes

Implement src/output/diff.rs with DiffFormatter struct and DiffFormat enum (Unified, SideBySide). Methods: format_diff(old: &str, new: &str, format: DiffFormat) -> String. Unified format: standard diff with +/- prefixes, @@ chunk headers, context lines. Side-by-side format: two-column layout with | separator, aligned changes. Color coding: green for additions, red for deletions, cyan for context. Handle line wrapping and terminal width constraints. Include similar crate for diff generation. Add --diff-format flag parsing.

**Rationale:** Diff display for improve command requires specialized formatting distinct from other outputs

## T-34: Pager integration with configurable options

Create src/output/pager.rs with PagerManager struct. Methods: should_use_pager(content_length: usize, force_disable: bool) -> bool, pipe_through_pager(content: String). Read $PAGER environment variable (default: 'less -R' for color support). Only activate pager when stdout is TTY and content exceeds terminal height. Implement --no-pager flag override. Handle pager process spawning, stdin piping, and error cases (pager not found, broken pipe). Graceful fallback to direct output when pager fails. Include subprocess handling with proper signal management.

**Rationale:** Pager functionality needs separate implementation with process management complexity

## T-35: OutputManager orchestration layer with format dispatch

Implement src/output/mod.rs with OutputManager struct that coordinates all formatters. Methods: render<T>(items: Vec<T>, format_flags: FormatFlags) -> Result<String> with generic trait bounds for different item types. Dispatch logic to appropriate renderer based on OutputFormat. Integrate terminal detection, color management, and pager decisions. Handle format-specific trait implementations (TableRow, CsvRow, Serialize). Add render_detail() method for single-item views with markdown. Include error handling for rendering failures and unsupported format combinations. Export all sub-modules and provide unified API for CLI commands.

**Rationale:** Orchestration layer needed to coordinate all formatting components and provide clean API

## F-2: Project Commands

Commands for listing and inspecting projects within the authenticated org.

**Commands:**
- `funcspec projects list` — List all accessible projects. Columns: slug, name, item count, last updated.
- `funcspec projects show <slug>` — Show project details including stats (func/tech counts, review coverage, implementation status breakdown).
- `funcspec projects set-default <slug>` — Shorthand for `funcspec config set project <slug>`. Sets the default project context so `--project` flag is optional on other commands.

**Output formats** (applies globally, see F-6):
- Default: human-readable table
- `--json` flag: machine-readable JSON
- `--quiet` flag: IDs/slugs only (for piping)

**Notes:**
- Project context is required for most item commands. If no default is set and no `--project` flag provided, show a helpful error listing available projects.
- Accept both slug (`funcspec-cli`) and org-qualified slug (`tambit/funcspec-cli`).

## T-44: Project data model and API client methods

Extend src/models/mod.rs with Project struct containing id (String), slug (String), name (String), org_slug (String), item_count (u32), last_updated (DateTime<Utc>), func_count (u32), tech_count (u32), review_coverage (f32), implementation_status (HashMap<String, u32>). Add ProjectListResponse and ProjectDetailResponse wrapper structs. In src/client.rs, implement list_projects() -> Result<Vec<Project>>, get_project(slug: &str) -> Result<Project>, and parse_project_slug(input: &str) -> (Option<String>, String) helper to handle both 'slug' and 'org/slug' formats. API endpoints: GET /api/v1/projects for list, GET /api/v1/projects/{slug} for details. Handle 404 errors gracefully with custom ProjectNotFound error variant.

**Rationale:** Core data model and API communication layer needed before CLI commands can be implemented

## T-45: Project configuration management with default project setting

Extend src/config.rs Config struct with default_project (Option<String>) field. Add set_default_project(slug: String) -> Result<()> and get_default_project() -> Option<String> methods. Update save() method to persist default_project to config file. In CLI parsing, add logic to resolve project context: 1) Use --project flag if provided, 2) Fall back to default_project from config, 3) Show helpful error with available projects list if neither exists. Add validate_project_access(client: &FuncSpecClient, slug: &str) -> Result<()> helper to verify project exists and user has access.

**Rationale:** Configuration management is separate concern from API operations and needed for project context resolution

## T-46: Projects list command with formatted output

Create src/commands/projects/list.rs with run_projects_list(client: &FuncSpecClient, format: OutputFormat) -> Result<()>. Fetch projects via client.list_projects(), sort by name. For table format: use tabled crate with columns [Slug, Name, Items, Last Updated]. Format item_count as number, last_updated as relative time (e.g. '2 days ago'). For JSON format: serialize Vec<Project> directly. For quiet format: output only slugs, one per line. Handle empty projects list gracefully. Add --org filter option to show only projects from specific org. Include error handling for network failures and auth errors.

**Rationale:** List command is distinct UI concern with specific formatting requirements

## T-47: Project show command with detailed statistics

Create src/commands/projects/show.rs with run_project_show(client: &FuncSpecClient, slug: String, format: OutputFormat) -> Result<()>. Use parse_project_slug() to handle org-qualified slugs. Fetch project details via client.get_project(). For table format: display project info (name, slug, org), then stats section (func items, tech items, total items), then implementation status breakdown as sub-table (status -> count). For JSON format: serialize Project struct with all nested data. For quiet format: output only project ID. Add validation that project exists with helpful error message suggesting similar project names on 404.

**Rationale:** Show command has different data requirements and formatting logic from list command

## T-48: Set default project command integration

Create src/commands/projects/set_default.rs with run_set_default_project(client: &FuncSpecClient, config: &mut Config, slug: String) -> Result<()>. Validate project exists and is accessible via validate_project_access(). Use parse_project_slug() to normalize slug format. Call config.set_default_project() and config.save(). Output success message with project name. Handle errors: project not found, no access, config save failures. Update main CLI parser to route 'funcspec projects set-default <slug>' to this command. Add this as alias for 'funcspec config set project <slug>' in config command routing.

**Rationale:** Set-default command bridges project operations and configuration management, requiring separate implementation

## T-49: Projects command router and CLI integration

Create src/commands/projects/mod.rs with ProjectsCommand enum (List, Show { slug: String }, SetDefault { slug: String }) and run_projects_command(cmd: ProjectsCommand, client: &FuncSpecClient, config: &mut Config, format: OutputFormat) -> Result<()> dispatcher. Update main.rs CLI parser with projects subcommand using clap derive macros. Add global --project flag parsing and context resolution logic. Integrate project context validation for commands that require it. Add helpful error messages when project context is missing, including list of available projects. Export all projects subcommands from mod.rs.

**Rationale:** Command routing and CLI integration requires coordinating all project subcommands and global options

## F-3: Item CRUD Commands

Core commands for creating, reading, updating, and managing spec items (functional and technical).

**List items:**
- `funcspec items list` — List all items in the current project
- `funcspec items list --type func|tech` — Filter by type
- `funcspec items list --status not_started|in_progress|implemented` — Filter by implementation status
- `funcspec items list --tag v1` — Filter by tag
- `funcspec items list --q "search term"` — Full-text search
- `funcspec items list --has-review` — Only items with reviews
- `funcspec items list --review-verdict pass|needs_refinement|major_gaps` — Filter by review result
- `funcspec items list --parent F-100` — List children of a specific item
- Pagination: `--page N --per N` (default 25)

**Show item:**
- `funcspec items show F-377` — Show full item detail: title, description, status, tags, review summary, parent/children, related items
- Accept permalink (F-377), numeric ID, or title substring match

**Create item:**
- `funcspec items create --title "..." --type func` — Create new spec item
- `funcspec items create --title "..." --type tech --parent F-100` — Create tech item under a functional parent
- `--description` flag or open `$EDITOR` if omitted (like `git commit`)
- `--tag v1,core` — Comma-separated tags

**Update item:**
- `funcspec items update F-377 --status implemented` — Update implementation status (respects state machine: not_started → in_progress → implemented)
- `funcspec items update F-377 --title "New title"` — Update fields
- `funcspec items update F-377 --tag v1,v2` — Replace tags
- `funcspec items update F-377 --description -` — Read description from stdin
- `funcspec items edit F-377` — Open item in `$EDITOR` as markdown, save to update

**Delete item:**
- `funcspec items delete F-377` — Delete item (with confirmation prompt, `--yes` to skip)

**Bulk operations:**
- `funcspec items list --type tech --status not_started --quiet | xargs -I{} funcspec items update {} --status in_progress` — Unix pipeline friendly

## T-50: Items CLI command structure and argument parsing

Implement the CLI command structure in funcspec-cli for all items operations. Create src/commands/items.rs with subcommands: ItemsCommand enum with List, Show, Create, Update, Edit, Delete variants. Each variant should have its own Args struct with clap derives. ListArgs: type (Optional<ItemType>), status (Optional<Status>), tag (Optional<String>), query (Optional<String>), has_review (bool), review_verdict (Optional<ReviewVerdict>), parent (Optional<String>), page (Option<u32>), per (Option<u32>). ShowArgs: identifier (String). CreateArgs: title (String), item_type (ItemType), parent (Optional<String>), description (Optional<String>), tags (Vec<String>). UpdateArgs: identifier (String), status (Optional<Status>), title (Optional<String>), tags (Optional<Vec<String>>), description (Optional<String>). EditArgs: identifier (String). DeleteArgs: identifier (String), yes (bool). Handle editor integration for description input when --description not provided. Include validation for status transitions (not_started → in_progress → implemented). Add --quiet flag support for Unix pipeline compatibility.

**Rationale:** CLI interface needs robust argument parsing and command structure before API integration

## T-51: Item data models with comprehensive field support

Extend src/models/mod.rs to include Item struct and related types. Item struct: id (String), title (String), description (String), item_type (ItemType enum: Functional, Technical), status (Status enum: NotStarted, InProgress, Implemented), tags (Vec<String>), parent_id (Option<String>), created_at (DateTime<Utc>), updated_at (DateTime<Utc>), review_summary (Option<ReviewSummary>). ReviewSummary struct: verdict (ReviewVerdict enum: Pass, NeedsRefinement, MajorGaps), notes (Option<String>), reviewed_at (DateTime<Utc>). ItemType and Status enums with serde derives and Display traits. Implement methods: can_transition_to_status(&self, new_status: Status) -> bool for state machine validation. Add ItemsListResponse struct for paginated API responses: items (Vec<Item>), total_count (usize), page (u32), per_page (u32), has_more (bool). Include ItemIdentifier enum to handle permalink (F-377), numeric ID, or title matching.

**Rationale:** Domain models must support all fields and relationships for items including reviews and hierarchy

## T-52: Items API client methods with comprehensive filtering and operations

Extend FuncSpecClient in src/client.rs with item management methods. list_items(&self, project_id: &str, filters: ItemsListFilters) -> Result<ItemsListResponse> with ItemsListFilters struct containing all filter options. show_item(&self, project_id: &str, identifier: &ItemIdentifier) -> Result<Item> with smart identifier resolution (try permalink, then ID, then title substring). create_item(&self, project_id: &str, request: CreateItemRequest) -> Result<Item> with CreateItemRequest containing all creation fields. update_item(&self, project_id: &str, identifier: &ItemIdentifier, request: UpdateItemRequest) -> Result<Item> with partial update support and status transition validation. delete_item(&self, project_id: &str, identifier: &ItemIdentifier) -> Result<()>. Include proper URL construction for REST endpoints: GET /projects/{id}/items with query parameters, POST /projects/{id}/items, PUT /projects/{id}/items/{item_id}, DELETE /projects/{id}/items/{item_id}. Handle pagination headers and response metadata. Add error handling for item not found, invalid transitions, and validation failures.

**Rationale:** API client needs full CRUD operations with advanced filtering and identifier resolution

## T-53: Editor integration utilities for markdown content editing

Create src/utils/editor.rs with editor integration functions. open_editor_for_content(existing_content: Option<&str>) -> Result<String> that: detects $EDITOR environment variable (fallback to 'vi'), creates temporary file with .md extension, writes existing content if provided, spawns editor process and waits for completion, reads back the content, cleans up temporary file, returns edited content or error. Handle edge cases: empty editor (return error), editor exits with non-zero code, file read/write permissions, SIGINT during editing. Add format_item_for_editing(item: &Item) -> String to convert Item to markdown format for the edit command: structured markdown with frontmatter-style metadata (title, type, status, tags, parent) and description as markdown body. parse_edited_content(content: &str) -> Result<EditedItem> to parse the markdown back into structured data with validation.

**Rationale:** Editor integration is a separate concern requiring temporary file handling and markdown formatting

## T-54: Items command handlers with business logic and user interaction

Implement command execution logic in src/commands/items.rs. handle_list_command(client: &FuncSpecClient, project_id: &str, args: ListArgs) -> Result<()> with formatted table output using prettytable or similar, pagination controls, quiet mode for Unix pipelines. handle_show_command() with detailed item display including parent/children relationships and review information. handle_create_command() with editor integration when description not provided, tag parsing, parent validation. handle_update_command() with status transition validation, stdin reading for --description -, field merging. handle_edit_command() integrating editor utils, parsing edited content, validating changes. handle_delete_command() with confirmation prompt unless --yes flag, cascade deletion warnings for parent items. Include proper error handling and user-friendly error messages. Add progress indicators for long operations. Format output consistently with colors and formatting (using termcolor or similar). Support JSON output format with --json flag for programmatic use.

**Rationale:** Command handlers contain the business logic and user interaction patterns separate from CLI parsing and API calls
