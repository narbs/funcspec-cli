=== SEARCH & FILTERING SPECS ===

## F-4: Search & Filtering

Rich search and filtering capabilities that leverage the API search/filter endpoints.

**Dedicated search command:**
- `funcspec search "authentication"` — Search across all items in current project
- `funcspec search "auth" --type func` — Scoped search
- Results show: permalink, title, type, status, relevance snippet

**Filter combinations on `items list`:**
- All filters are composable: `funcspec items list --type tech --status implemented --tag v1 --has-review`
- `--sort field:asc|desc` — Sort by: title, created_at, updated_at, permalink, coverage_score
- `--since 2026-03-01` — Items updated since date

**Output piping:**
- `--quiet` outputs only permalinks (one per line) for piping to other commands
- `--json` outputs full JSON array for `jq` processing
- `--csv` outputs CSV format for spreadsheet import
- `--count` outputs just the count of matching items
- `--bare` outputs tab-separated values without table borders, headers, or decoration — ideal for grep, awk, cut, and other Unix text processing. Columns match the table view (permalink, type, title, status, score) separated by tabs.

**Examples:**
```
funcspec items list --type tech --review-verdict major_gaps --quiet | wc -l
funcspec items list --json | jq ".[].title"
funcspec search "export" --json | jq -r ".[] | [.permalink, .title] | @tsv"
funcspec items list --bare | grep implemented | cut -f1,3
funcspec items list --bare --type func | awk -F"	" "$4 == \"not_started\""
```

## T-55: Search API endpoint methods in FuncSpecClient

Add search functionality to src/client.rs FuncSpecClient implementation. Add method `search(&self, project_id: &str, query: &str, item_type: Option<ItemType>) -> Result<Vec<SearchResult>, Error>` that calls GET /projects/{project_id}/search with query parameters: q (required), type (optional). Implement SearchResult model in models/mod.rs with fields: id, permalink, title, item_type, status, relevance_score, snippet. Handle API response parsing and error cases (400 for invalid query, 404 for project not found). Include comprehensive unit tests for different search scenarios and error conditions.

**Rationale:** Core API client functionality needed before CLI can implement search commands

## T-56: Enhanced filtering parameters for list_items API method

Extend existing list_items method in src/client.rs to support comprehensive filtering. Add ListItemsOptions struct with fields: item_type: Option<ItemType>, status: Option<Status>, tags: Option<Vec<String>>, has_review: Option<bool>, review_verdict: Option<ReviewVerdict>, sort_field: Option<SortField>, sort_direction: Option<SortDirection>, since: Option<chrono::DateTime<chrono::Utc>>. Update list_items signature to `list_items(&self, project_id: &str, options: Option<ListItemsOptions>) -> Result<Vec<Item>, Error>`. Build query parameters dynamically, handling multiple tags as repeated parameters, date formatting for since filter. Add SortField enum (Title, CreatedAt, UpdatedAt, Permalink, CoverageScore) and SortDirection enum (Asc, Desc) to models. Include tests for all filter combinations.

**Rationale:** API layer must support all filtering options before CLI commands can use them

## T-57: Search command implementation in CLI

Implement `funcspec search` subcommand in src/main.rs. Add SearchArgs struct with fields: query (required String), item_type (optional), output_format (OutputFormat enum). Parse --type flag to filter by ItemType. Call client.search() and format results as table by default showing: permalink (truncated to fit), title (truncated), type, status, relevance snippet (truncated to 50 chars). Support --json and --quiet output formats. Handle errors gracefully with user-friendly messages. Add color coding for different item types and statuses using console crate. Include comprehensive help text and examples in clap derive annotations.

**Rationale:** Dedicated search command is a separate user interface concern from listing/filtering

## T-58: Enhanced items list command with filtering and output options

Extend existing `funcspec items list` command in src/main.rs to support all filtering flags. Add fields to ListArgs: status, tags (Vec<String>), has_review, review_verdict, sort, since. Add OutputFormat enum (Table, Json, Csv, Quiet, Count, Bare) with corresponding --json, --csv, --quiet, --count, --bare flags. Implement format_items_output function that handles each format: Table uses comfy-table, Json uses serde_json, Csv uses csv crate, Quiet outputs permalinks only, Count outputs just the number, Bare outputs TSV without borders/headers. For --tags flag, allow multiple values and comma-separated lists. Parse --sort as field:direction format. Add --since with date parsing using chrono. Include input validation and error handling for all parameters.

**Rationale:** List command enhancements require different formatting logic and flag parsing from search

## T-59: Output formatting utilities and CSV/TSV export functionality

Create src/output.rs module with formatting utilities. Implement OutputFormatter trait with methods: format_table, format_json, format_csv, format_bare, format_quiet, format_count. Add dependencies: csv crate for CSV output, comfy-table for enhanced table formatting. For CSV format, include headers: permalink,type,title,status,coverage_score,created_at,updated_at. For bare format, output tab-separated values without any decoration, suitable for Unix text processing. Implement proper escaping for CSV fields containing commas/quotes. Add utility functions for truncating text in table view, color coding status values, and formatting dates consistently across all output formats. Include unit tests for each format type.

**Rationale:** Output formatting is complex enough to warrant separate module and is reused across commands
