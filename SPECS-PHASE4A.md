=== AI OPERATIONS SPECS ===

## F-5: AI Operations

Commands for triggering AI-powered review, refinement, generation, and audit operations.

**Review:**
- `funcspec review F-377` — Trigger AI review of a spec item. Show results inline: score, verdict, coverage map, gaps, suggestions.
- `funcspec review --all` — Trigger batch review of all stale/unreviewed items. Shows progress and summary.
- `funcspec review F-377 --show` — Show existing review without re-running.

**Improve/Refine:**
- `funcspec improve F-377` — Trigger AI improvement proposal. Show side-by-side diff in terminal.
- `funcspec improve F-377 --accept` — Accept the proposed changes.
- `funcspec improve F-377 --reject` — Reject the proposal.
- `funcspec improve --all` — Batch improve all items needing refinement.

**Generate tech specs:**
- `funcspec generate F-100` — Generate tech specs from a functional item. Show proposed items.
- `funcspec generate F-100 --accept` — Accept and create all proposed tech items.
- `funcspec generate F-100 --interactive` — Review and accept/reject each proposed item individually.

**Audit:**
- `funcspec audit F-377` — Run code audit against repo source. Show coverage, evidence, confidence.
- `funcspec audit --all` — Batch audit.

**Job tracking:**
- AI operations are async. CLI should poll for completion with a spinner/progress indicator.
- `funcspec jobs list` — Show pending/running jobs.
- `funcspec jobs show <id>` — Show job detail and result.
- `funcspec jobs cancel <id>` — Cancel a running job.

**Notes:**
- All AI ops consume LLM tokens from the org's configured provider. Show token usage in output when available.
- `--dry-run` flag shows what would be sent without executing.

## T-60: AI operations data models with job tracking and result serialization

Extend src/models/mod.rs with AI operation models. Add AIOperation enum (Review, Improve, Generate, Audit) with operation-specific parameters. Add Job struct: id (String), operation (AIOperation), status (JobStatus enum: Pending, Running, Completed, Failed, Cancelled), created_at (DateTime), completed_at (Option<DateTime>), result (Option<serde_json::Value>), error (Option<String>), token_usage (Option<TokenUsage>). Add TokenUsage struct: input_tokens (u32), output_tokens (u32), total_cost (Option<f64>). Add operation result models: ReviewResult (score: f64, verdict: String, coverage_map: Vec<String>, gaps: Vec<String>, suggestions: Vec<String>), ImproveResult (original: String, improved: String, changes: Vec<Change>), GenerateResult (proposed_specs: Vec<TechSpec>), AuditResult (coverage_percentage: f64, evidence: Vec<Evidence>, confidence: f64). Add Change struct for diff representation: line_number (u32), change_type (ChangeType enum), old_content (Option<String>), new_content (Option<String>). All structs derive Serialize, Deserialize, Debug, Clone.

**Rationale:** AI operations require complex data structures for jobs, results, and tracking that extend beyond basic CRUD models

## T-61: Extended HTTP client with async AI operation endpoints

Extend FuncSpecClient in src/client.rs with AI operation methods. Add review_spec(spec_id: &str, force: bool) -> Result<Job>. Add batch_review_specs(filters: Option<ReviewFilters>) -> Result<Job>. Add get_review_result(spec_id: &str) -> Result<Option<ReviewResult>>. Add improve_spec(spec_id: &str) -> Result<Job>. Add accept_improvement(spec_id: &str, job_id: &str) -> Result<()>. Add reject_improvement(spec_id: &str, job_id: &str) -> Result<()>. Add batch_improve_specs() -> Result<Job>. Add generate_tech_specs(func_spec_id: &str) -> Result<Job>. Add accept_generated_specs(job_id: &str, spec_ids: Vec<String>) -> Result<()>. Add audit_spec(spec_id: &str) -> Result<Job>. Add batch_audit_specs() -> Result<Job>. Add list_jobs(status_filter: Option<JobStatus>) -> Result<Vec<Job>>. Add get_job(job_id: &str) -> Result<Job>. Add cancel_job(job_id: &str) -> Result<()>. Add poll_job_completion(job_id: &str, timeout: Duration) -> Result<Job>. All methods handle authentication headers and return proper error types.

**Rationale:** AI operations require specialized HTTP endpoints with async job handling separate from standard CRUD operations

## T-62: AI operations command handlers with job polling and progress display

Create src/commands/ai_ops.rs with command handlers for AI operations using the indicatif crate for progress indicators, similar crate for diff display, comfy-table for result formatting, and serde_json for dry-run previews.

Implement ReviewCommand with handle_review(spec_id: Option<String>, all: bool, show: bool, dry_run: bool):
- For single spec review: Display dry-run preview showing API call parameters if --dry-run, otherwise call client.review_spec() with retry logic (3 attempts with exponential backoff), poll job status every 500ms with indicatif::ProgressBar spinner showing "Analyzing specification...", display ReviewResult in formatted table with score (0-100), verdict (Pass/Fail/Needs Work), and bulleted gaps list
- For --all flag: Call batch_review_specs(), show indicatif::ProgressBar with percentage and current spec name, handle individual failures gracefully by logging errors and continuing
- For --show flag: Call get_review_result() and display cached results in same format, show "No review found" if none exists
- Error handling: Catch API timeouts, authentication failures, and network errors with user-friendly messages and retry prompts

Implement ImproveCommand with handle_improve(spec_id: Option<String>, accept: bool, reject: bool, all: bool):
- Load original spec and call client.improve_spec() with progress spinner
- Display side-by-side diff using similar::TextDiff with original (left) and improved (right) versions, highlighting additions in green and deletions in red
- For interactive mode: Show "Accept changes? [y/n/q]", 'y' saves improved version, 'n' discards, 'q' quits
- For --accept/--reject flags: Apply action immediately without prompts
- Implement confirmation prompt "This will overwrite the existing specification. Continue? [y/N]" for destructive operations

Implement GenerateCommand with handle_generate(func_spec_id: String, accept: bool, interactive: bool):
- Call client.generate_tech_specs() with progress bar showing "Generating technical specifications..."
- Display results using comfy-table with columns: ID, Title, Complexity (Low/Med/High), Confidence (0-100%)
- For --interactive mode: Show each spec with "Accept this specification? [y/n/s/q]" where 's' shows full spec content, implement arrow key navigation
- For --accept flag: Accept all generated specs automatically with confirmation prompt
- Cache generation results in ~/.specr/cache/generated_{func_spec_id}.json for 24 hours

Implement AuditCommand with handle_audit(spec_id: Option<String>, all: bool):
- Call client.audit_coverage() and display results table with columns: Requirement, Coverage %, Evidence Count, Confidence
- Show overall coverage percentage as colored progress bar (red <50%, yellow 50-80%, green >80%)
- List evidence items as bulleted list with source references
- For --all flag: Show summary table of all specs with aggregate coverage metrics

Shared implementation details:
- Token usage display: Show "Tokens used: {input}/{output} (cost: ${estimated})" at operation completion
- Dry-run mode: Display formatted JSON preview with API endpoint, parameters, and estimated token usage using serde_json::to_string_pretty
- Error handling: Implement comprehensive error types (ApiError, NetworkError, AuthError) with context-specific retry logic and user guidance
- Caching: Store API results in ~/.specr/cache/ with TTL metadata, check cache before API calls
- Concurrent operations: Use tokio::sync::Semaphore to limit concurrent AI API calls to 3, show queued operations in progress display
- Keyboard shortcuts: Implement 'q' to quit, 'h' for help, arrow keys for navigation in interactive modes using crossterm crate
- User confirmations: All destructive operations require explicit confirmation with clear consequence descriptions

## T-63: Job management command handlers with status tracking

Create src/commands/jobs.rs with job management functionality. Implement JobsCommand with subcommands: list, show, cancel. For list subcommand: call client.list_jobs(), display jobs in table format with columns: ID, Operation, Status, Created, Duration, Token Usage. Support status filtering. For show subcommand: call client.get_job(), display detailed job information including full result data formatted appropriately for operation type (review results as bullet points, improve results as diff, generate results as spec list, audit results as coverage report). For cancel subcommand: call client.cancel_job(), confirm cancellation and show updated status. Add utility functions: format_job_duration(), format_token_usage(), format_job_status_with_color(). Handle error cases: job not found, job already completed, cancellation not allowed for completed jobs.

**Rationale:** Job management requires separate command structure with specialized formatting and status handling logic

## T-64: Terminal UI utilities for progress indication and diff display

Create src/ui/mod.rs with UI utility functions for AI operations. Implement JobProgressSpinner struct using indicatif crate: new(message: &str), update_message(&mut self, msg: &str), finish_with_message(&mut self, msg: &str). Implement BatchProgressBar struct: new(total: u64), increment(&mut self), set_message(&mut self, msg: &str), finish(&mut self). Implement DiffDisplay struct using similar crate for side-by-side diffs: new(original: &str, improved: &str), render() -> String. Add format_review_result(result: &ReviewResult) -> String with colored output: green score if >7, yellow if 5-7, red if <5. Format gaps and suggestions as bulleted lists. Add format_audit_result(result: &AuditResult) -> String with coverage bar chart and evidence formatting. Add format_token_usage(usage: &TokenUsage) -> String with cost display if available. All functions use console crate for color support and handle terminal width for proper formatting.

**Rationale:** AI operations require rich terminal UI components for progress indication and result display that are reusable across commands

## T-65: CLI argument parsing and routing for AI operation commands

Extend src/cli.rs with AI operation subcommands. Add AiCommand enum with Review, Improve, Generate, Audit, Jobs variants. For Review variant: spec_id (Option<String>), all (bool), show (bool), dry_run (bool). For Improve variant: spec_id (Option<String>), accept (bool), reject (bool), all (bool), dry_run (bool). For Generate variant: func_spec_id (String), accept (bool), interactive (bool), dry_run (bool). For Audit variant: spec_id (Option<String>), all (bool), dry_run (bool). For Jobs variant: subcommand (JobsSubcommand enum with List, Show, Cancel). Add JobsSubcommand with List { status: Option<JobStatus> }, Show { job_id: String }, Cancel { job_id: String }. Update main Command enum to include Ai(AiCommand). Update command routing in main.rs to handle AI operations with proper error handling and client initialization. Add validation: ensure spec_id format is valid, prevent conflicting flags (accept + reject), require confirmation for batch operations.

**Rationale:** AI operations require extensive CLI argument structures and validation logic separate from basic CRUD command parsing
