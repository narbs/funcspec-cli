=== STATS & DASHBOARD SPECS ===

## F-8: Stats & Dashboard Command

Quick project health overview from the command line.

**Command:**
- `funcspec stats` — Show project dashboard for current project

**Output includes:**
- Total items (functional / technical breakdown)
- Implementation status breakdown (count + percentage per status)
- Review coverage (reviewed vs unreviewed, avg score)
- Review verdict distribution (pass / needs_refinement / major_gaps)
- Tag summary (count per tag)
- LLM usage summary (tokens used this month, estimated cost)
- Recent activity (last 5 updated items)

**Flags:**
- `--json` for machine-readable output
- `--usage` — Focus on LLM usage stats
- `--month 2026-03` — Usage for a specific month

**Example output:**
```
FuncSpec CLI (tambit/funcspec-cli)
──────────────────────────────────
Items:     42 total (12 functional, 30 technical)
Status:    28 implemented (66.7%) │ 8 in progress │ 6 not started
Reviews:   35 reviewed (83.3%) │ avg score 87.2
Verdicts:  20 pass │ 12 needs refinement │ 3 major gaps
Usage:     45.2k tokens this month (~$0.12)
Last updated: F-5 AI Operations (2 hours ago)
```

## T-72: Stats aggregation methods in FuncSpecClient with dashboard data fetching

Extend FuncSpecClient in src/client.rs with stats aggregation methods. Add get_project_stats(project_id: &str) -> Result<ProjectStats> that fetches aggregated dashboard data from API. Include get_usage_stats(project_id: &str, month: Option<&str>) -> Result<UsageStats> for LLM usage metrics. Handle API responses for stats endpoints, parse JSON into strongly typed structs. Implement error handling for missing projects or invalid date formats. Add HTTP request builders with proper authentication headers and query parameters for month filtering.

**Rationale:** Separate API client logic from CLI presentation - follows existing pattern of resource-specific client methods

## T-73: ProjectStats and UsageStats models with statistics service implementation

Add ProjectStats and UsageStats structs to src/models/mod.rs with comprehensive statistics calculation and persistence implementation.

**Data Models:**
ProjectStats fields: total_items (u32), functional_count (u32), technical_count (u32), status_breakdown (HashMap<String, u32>), review_coverage (ReviewCoverage), verdict_distribution (VerdictDistribution), tag_summary (HashMap<String, u32>), recent_activity (Vec<RecentActivity>), last_updated (DateTime<Utc>). UsageStats fields: month (String), total_tokens (u32), estimated_cost (f64), breakdown_by_operation (HashMap<String, TokenUsage>), last_updated (DateTime<Utc>). Nested structs: ReviewCoverage { reviewed_count: u32, total_count: u32, avg_score: Option<f64> }, VerdictDistribution { pass: u32, needs_refinement: u32, major_gaps: u32 }, RecentActivity { item_id: String, item_title: String, updated_at: DateTime<Utc>, activity_type: String }, TokenUsage { tokens: u32, cost: f64 }.

**Statistics Service:**
Add StatisticsService struct with methods: calculate_project_stats(project_id: &str) -> Result<ProjectStats, StatisticsError>, calculate_usage_stats(month: &str) -> Result<UsageStats, StatisticsError>, update_activity_log(activity: ActivityEvent), get_cached_stats(project_id: &str) -> Option<ProjectStats>. Include StatisticsRepository trait with implementations for database persistence: save_project_stats, load_project_stats, save_usage_stats, load_usage_stats, get_recent_activities.

**Cost Calculation:**
Add CostCalculator struct with configurable pricing rules per operation type (review, analysis, generation). Include methods: calculate_operation_cost(operation_type: &str, tokens: u32) -> f64, get_monthly_breakdown(activities: &[ActivityEvent]) -> HashMap<String, TokenUsage>.

**Activity Tracking:**
Add ActivityTracker component that captures item updates, review completions, and system operations. Include ActivityEvent struct { project_id: String, item_id: String, activity_type: String, timestamp: DateTime<Utc>, token_usage: Option<u32> } for feeding statistics calculations.

**Data Aggregation:**
Implement StatisticsAggregator with methods for real-time calculation from existing project data: aggregate_item_counts, calculate_review_metrics, compute_verdict_distribution, generate_activity_feed. Include caching layer with configurable TTL to balance accuracy and performance.

**Error Handling:**
Define StatisticsError enum with variants: DatabaseError, CalculationError, InvalidInput, CacheError. All statistics operations return Results with proper error propagation.

**Database Schema:**
Add migrations for project_statistics and usage_statistics tables with proper indexing on project_id and month columns. Include activity_log table for tracking events.

All structs derive Serialize, Deserialize, Debug, Clone with additional validation traits where appropriate.

## T-74: Stats command handler with formatted dashboard output

Create src/commands/stats.rs with StatsCommand struct and StatsArgs for clap parsing. Fields: json (bool), usage_only (bool), month (Option<String>). Implement async execute() method that: 1) Gets current project from config, 2) Calls client.get_project_stats() and optionally get_usage_stats(), 3) Formats output based on flags. For default output, create formatted dashboard with project header, item counts, status breakdown with percentages, review metrics, verdict distribution, tag summary, usage stats, and recent activity list. For --json flag, serialize complete stats to JSON. For --usage flag, show only LLM usage metrics with month breakdown. Handle month validation (YYYY-MM format). Add colored output using colored crate for status indicators and percentages. Include error handling for missing project config and API failures.

**Rationale:** Command-specific logic separated from client and models - follows existing command structure pattern

## T-75: Dashboard formatting utilities with colored output and progress indicators

Create src/utils/formatting.rs with dashboard formatting functions. Implement format_dashboard(stats: &ProjectStats, usage: Option<&UsageStats>) -> String that creates the formatted dashboard output. Include helper functions: format_status_breakdown() with colored percentages and progress bars, format_review_coverage() with percentage calculations, format_verdict_distribution() with colored status indicators, format_usage_stats() with token counts and cost formatting, format_recent_activity() with relative timestamps. Add format_percentage(count: u32, total: u32) -> String for consistent percentage display. Use colored crate for green (implemented), yellow (in progress), red (not started) status colors. Include progress bar visualization using Unicode block characters. Add number formatting for large token counts (45.2k format) and currency formatting for costs ($0.12 format).

**Rationale:** Formatting logic extracted to utilities for reusability and testability - keeps command handler focused on business logic

## T-76: Stats command integration in CLI with argument parsing

Add StatsCommand to src/main.rs Commands enum and integrate with clap parsing. Update match statement in main() to handle Commands::Stats case by calling stats_command.execute(). Add stats subcommand to clap App with arguments: --json (bool flag for JSON output), --usage (bool flag for usage-only display), --month (optional string value with validation). Include help text and examples for each flag. Add month format validation in clap using value_parser to ensure YYYY-MM format. Set up error handling to provide helpful messages for invalid month formats or missing project context. Import StatsCommand and related types from commands::stats module.

**Rationale:** CLI integration follows existing command pattern - keeps argument parsing and routing consistent with other commands
