use std::io::{self, IsTerminal};

use anyhow::Result;
use clap::ValueEnum;
use colored::Colorize;
use comfy_table::{Attribute, Cell, ContentArrangement, Table};
use funcspec_client::models::*;

// ---------------------------------------------------------------------------
// OutputFormat
// ---------------------------------------------------------------------------

/// Output format for CLI commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum OutputFormat {
    /// Auto-detect: table when stdout is a TTY, JSON when piped
    #[default]
    Auto,
    /// Human-readable table with colored headers
    Table,
    /// JSON (pretty-printed)
    Json,
    /// CSV with headers
    Csv,
    /// Minimal one-line per item (permalink/slug + title)
    Minimal,
    /// Markdown (headers, bold, lists)
    Markdown,
    /// Tab-separated values without borders or headers (pipe-friendly: grep, awk, cut)
    Bare,
}

impl OutputFormat {
    /// Resolve `Auto` to a concrete format based on TTY detection.
    pub fn resolve(self) -> Self {
        match self {
            OutputFormat::Auto => {
                if io::stdout().is_terminal() {
                    OutputFormat::Table
                } else {
                    OutputFormat::Json
                }
            }
            other => other,
        }
    }
}

// ---------------------------------------------------------------------------
// Terminal width
// ---------------------------------------------------------------------------

/// Return current terminal width, or 80 if not detectable.
pub fn terminal_width() -> usize {
    terminal_size::terminal_size()
        .map(|(terminal_size::Width(w), _)| w as usize)
        .unwrap_or(80)
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

/// Format a list of projects according to `format`.
pub fn format_projects(projects: &[Project], format: OutputFormat) -> Result<()> {
    match format.resolve() {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(projects)?);
        }
        OutputFormat::Csv => {
            let mut wtr = csv::Writer::from_writer(io::stdout());
            wtr.write_record(["slug", "name", "description", "created_at", "updated_at"])?;
            for p in projects {
                wtr.write_record([
                    p.attributes.slug.as_str(),
                    p.attributes.name.as_str(),
                    p.attributes.description.as_deref().unwrap_or(""),
                    &p.attributes
                        .created_at
                        .format("%Y-%m-%dT%H:%M:%SZ")
                        .to_string(),
                    &p.attributes
                        .updated_at
                        .format("%Y-%m-%dT%H:%M:%SZ")
                        .to_string(),
                ])?;
            }
            wtr.flush()?;
        }
        OutputFormat::Minimal => {
            for p in projects {
                println!("{}\t{}", p.attributes.slug, p.attributes.name);
            }
        }
        OutputFormat::Markdown => {
            println!("# Projects\n");
            for p in projects {
                println!("## {}", p.attributes.name);
                println!("- **Slug**: `{}`", p.attributes.slug);
                if let Some(ref desc) = p.attributes.description {
                    println!("- **Description**: {desc}");
                }
                println!(
                    "- **Updated**: {}",
                    p.attributes.updated_at.format("%Y-%m-%d")
                );
                println!();
            }
        }
        OutputFormat::Bare => {
            for p in projects {
                println!(
                    "{}\t{}\t{}",
                    p.attributes.slug,
                    p.attributes.name,
                    p.attributes.updated_at.format("%Y-%m-%d"),
                );
            }
        }
        _ => projects_table(projects),
    }
    Ok(())
}

/// Format a single project detail view.
pub fn format_project_detail(project: &Project, format: OutputFormat) -> Result<()> {
    match format.resolve() {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(project)?);
        }
        OutputFormat::Csv => {
            let mut wtr = csv::Writer::from_writer(io::stdout());
            wtr.write_record(["slug", "name", "description", "created_at", "updated_at"])?;
            let a = &project.attributes;
            wtr.write_record([
                a.slug.as_str(),
                a.name.as_str(),
                a.description.as_deref().unwrap_or(""),
                &a.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                &a.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            ])?;
            wtr.flush()?;
        }
        OutputFormat::Minimal => {
            let a = &project.attributes;
            println!("{}\t{}", a.slug, a.name);
        }
        OutputFormat::Markdown => {
            let a = &project.attributes;
            println!("## {}\n", a.name);
            println!("- **Slug**: `{}`", a.slug);
            if let Some(ref desc) = a.description {
                println!("- **Description**: {desc}");
            }
            println!("- **Created**: {}", a.created_at.format("%Y-%m-%d"));
            println!("- **Updated**: {}", a.updated_at.format("%Y-%m-%d"));
        }
        OutputFormat::Bare => {
            let a = &project.attributes;
            println!(
                "{}\t{}\t{}",
                a.slug,
                a.name,
                a.updated_at.format("%Y-%m-%d"),
            );
        }
        _ => {
            let a = &project.attributes;
            println!("{} {}", a.slug.cyan().bold(), a.name.as_str());
            if let Some(ref desc) = a.description {
                println!("{desc}");
            }
            println!("Created: {}", a.created_at.format("%Y-%m-%d"));
            println!("Updated: {}", a.updated_at.format("%Y-%m-%d"));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Items
// ---------------------------------------------------------------------------

/// Format a list of spec items according to `format`.
pub fn format_items(
    items: &[SpecItem],
    meta: Option<&PaginationMeta>,
    format: OutputFormat,
) -> Result<()> {
    match format.resolve() {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(items)?);
        }
        OutputFormat::Csv => {
            let mut wtr = csv::Writer::from_writer(io::stdout());
            wtr.write_record(["permalink", "type", "title", "status", "score", "tags"])?;
            for item in items {
                let a = &item.attributes;
                let score = a
                    .review
                    .as_ref()
                    .and_then(|r| r.coverage_score)
                    .map(|s| format!("{s:.0}"))
                    .unwrap_or_default();
                wtr.write_record([
                    a.permalink.as_str(),
                    &a.type_of.to_string(),
                    a.title.as_str(),
                    &a.implementation_status.to_string(),
                    score.as_str(),
                    &a.tags.join(","),
                ])?;
            }
            wtr.flush()?;
        }
        OutputFormat::Minimal => {
            for item in items {
                println!("{}\t{}", item.attributes.permalink, item.attributes.title);
            }
        }
        OutputFormat::Bare => {
            for item in items {
                let a = &item.attributes;
                let score = a
                    .review
                    .as_ref()
                    .and_then(|r| r.coverage_score)
                    .map(|s| format!("{s:.0}"))
                    .unwrap_or_default();
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    a.permalink, a.type_of, a.title, a.implementation_status, score
                );
            }
        }
        OutputFormat::Markdown => {
            println!("# Spec Items\n");
            for item in items {
                let a = &item.attributes;
                println!("## {} — {}", a.permalink, a.title);
                println!(
                    "- **Type**: {} | **Status**: {}",
                    a.type_of, a.implementation_status
                );
                if !a.tags.is_empty() {
                    println!("- **Tags**: {}", a.tags.join(", "));
                }
                println!();
            }
            if let Some(meta) = meta {
                println!("*Showing {} of {} items*", items.len(), meta.total);
            }
        }
        _ => items_table(items, meta),
    }
    Ok(())
}

/// Format a single spec item detail view.
pub fn format_item_detail(item: &SpecItem, format: OutputFormat) -> Result<()> {
    match format.resolve() {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(item)?);
        }
        OutputFormat::Csv => {
            let mut wtr = csv::Writer::from_writer(io::stdout());
            wtr.write_record([
                "permalink",
                "type",
                "title",
                "status",
                "description",
                "tags",
            ])?;
            let a = &item.attributes;
            wtr.write_record([
                a.permalink.as_str(),
                &a.type_of.to_string(),
                a.title.as_str(),
                &a.implementation_status.to_string(),
                a.description.as_deref().unwrap_or(""),
                &a.tags.join(","),
            ])?;
            wtr.flush()?;
        }
        OutputFormat::Minimal => {
            println!("{}\t{}", item.attributes.permalink, item.attributes.title);
        }
        OutputFormat::Bare => {
            let a = &item.attributes;
            let score = a
                .review
                .as_ref()
                .and_then(|r| r.coverage_score)
                .map(|s| format!("{s:.0}"))
                .unwrap_or_default();
            println!(
                "{}\t{}\t{}\t{}\t{}",
                a.permalink, a.type_of, a.title, a.implementation_status, score
            );
        }
        OutputFormat::Markdown => item_detail_markdown(item),
        _ => item_detail(item),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Edges
// ---------------------------------------------------------------------------

/// Format a list of dependency edges.
///
/// `item_map` maps item ID → (permalink, title) for display.
pub fn format_edges(
    edges: &[funcspec_client::models::DependencyEdge],
    item_map: &std::collections::HashMap<u64, (String, String)>,
    format: OutputFormat,
) -> Result<()> {
    match format.resolve() {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(edges)?);
        }
        OutputFormat::Csv => {
            let mut wtr = csv::Writer::from_writer(io::stdout());
            wtr.write_record([
                "id",
                "source_id",
                "source",
                "target_id",
                "target",
                "edge_type",
                "created_at",
            ])?;
            for edge in edges {
                let a = &edge.attributes;
                let (src_perm, src_title) = item_map
                    .get(&a.source_id)
                    .map(|(p, t)| (p.as_str(), t.as_str()))
                    .unwrap_or(("", ""));
                let (tgt_perm, tgt_title) = item_map
                    .get(&a.target_id)
                    .map(|(p, t)| (p.as_str(), t.as_str()))
                    .unwrap_or(("", ""));
                wtr.write_record([
                    &edge.id.to_string(),
                    &a.source_id.to_string(),
                    &format!("{src_perm} {src_title}"),
                    &a.target_id.to_string(),
                    &format!("{tgt_perm} {tgt_title}"),
                    &a.edge_type,
                    &a.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                ])?;
            }
            wtr.flush()?;
        }
        OutputFormat::Minimal => {
            for edge in edges {
                let a = &edge.attributes;
                let src = item_map
                    .get(&a.source_id)
                    .map(|(p, _)| p.as_str())
                    .unwrap_or("?");
                let tgt = item_map
                    .get(&a.target_id)
                    .map(|(p, _)| p.as_str())
                    .unwrap_or("?");
                println!("{}\t{}\t{}\t{}", edge.id, src, a.edge_type, tgt);
            }
        }
        OutputFormat::Bare => {
            for edge in edges {
                let a = &edge.attributes;
                let src = item_map
                    .get(&a.source_id)
                    .map(|(p, _)| p.as_str())
                    .unwrap_or("?");
                let tgt = item_map
                    .get(&a.target_id)
                    .map(|(p, _)| p.as_str())
                    .unwrap_or("?");
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    edge.id,
                    src,
                    a.edge_type,
                    tgt,
                    a.created_at.format("%Y-%m-%d"),
                );
            }
        }
        OutputFormat::Markdown => {
            println!("# Dependency Edges\n");
            for edge in edges {
                let a = &edge.attributes;
                let (src_perm, src_title) = item_map
                    .get(&a.source_id)
                    .map(|(p, t)| (p.as_str(), t.as_str()))
                    .unwrap_or(("?", ""));
                let (tgt_perm, tgt_title) = item_map
                    .get(&a.target_id)
                    .map(|(p, t)| (p.as_str(), t.as_str()))
                    .unwrap_or(("?", ""));
                println!(
                    "- **{}** `{}` {} → `{}` {} ({})",
                    edge.id, src_perm, src_title, tgt_perm, tgt_title, a.edge_type,
                );
            }
        }
        _ => edges_table(edges, item_map),
    }
    Ok(())
}

fn edges_table(
    edges: &[funcspec_client::models::DependencyEdge],
    item_map: &std::collections::HashMap<u64, (String, String)>,
) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("ID").add_attribute(Attribute::Bold),
        Cell::new("Source").add_attribute(Attribute::Bold),
        Cell::new("Type").add_attribute(Attribute::Bold),
        Cell::new("Target").add_attribute(Attribute::Bold),
        Cell::new("Created").add_attribute(Attribute::Bold),
    ]);

    for edge in edges {
        let a = &edge.attributes;
        let src = item_map
            .get(&a.source_id)
            .map(|(p, t)| format!("{} {}", p.cyan(), t))
            .unwrap_or_else(|| a.source_id.to_string());
        let tgt = item_map
            .get(&a.target_id)
            .map(|(p, t)| format!("{} {}", p.cyan(), t))
            .unwrap_or_else(|| a.target_id.to_string());
        table.add_row(vec![
            edge.id.to_string(),
            src,
            a.edge_type.clone(),
            tgt,
            a.created_at.format("%Y-%m-%d").to_string(),
        ]);
    }
    println!("{table}");
}

// ---------------------------------------------------------------------------
// Diff
// ---------------------------------------------------------------------------

/// Display a unified diff between `old` and `new` with +/- coloring.
pub fn format_diff(old: &str, new: &str) {
    use similar::{ChangeTag, TextDiff};

    let diff = TextDiff::from_lines(old, new);
    for change in diff.iter_all_changes() {
        let value: &str = change.value();
        match change.tag() {
            ChangeTag::Insert => print!("{}", format!("+{value}").green()),
            ChangeTag::Delete => print!("{}", format!("-{value}").red()),
            ChangeTag::Equal => print!(" {value}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn projects_table(projects: &[Project]) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("Slug").add_attribute(Attribute::Bold),
        Cell::new("Name").add_attribute(Attribute::Bold),
        Cell::new("Updated").add_attribute(Attribute::Bold),
    ]);
    for p in projects {
        table.add_row(vec![
            p.attributes.slug.clone(),
            p.attributes.name.clone(),
            p.attributes.updated_at.format("%Y-%m-%d").to_string(),
        ]);
    }
    println!("{table}");
}

fn items_table(items: &[SpecItem], meta: Option<&PaginationMeta>) {
    let width = terminal_width();
    // Reserve ~50 chars for other columns; title gets the remainder (min 30)
    let title_max = width.saturating_sub(50).max(30);

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("ID").add_attribute(Attribute::Bold),
        Cell::new("Type").add_attribute(Attribute::Bold),
        Cell::new("Title").add_attribute(Attribute::Bold),
        Cell::new("Status").add_attribute(Attribute::Bold),
        Cell::new("Score").add_attribute(Attribute::Bold),
    ]);

    for item in items {
        let type_str = match item.attributes.type_of {
            ItemType::Functional => "func".blue().to_string(),
            ItemType::Technical => "tech".magenta().to_string(),
        };
        let status_str = match item.attributes.implementation_status {
            ImplementationStatus::Implemented => "implemented".green().to_string(),
            ImplementationStatus::InProgress => "in_progress".yellow().to_string(),
            ImplementationStatus::NotStarted => "not_started".dimmed().to_string(),
        };
        let score = item
            .attributes
            .review
            .as_ref()
            .and_then(|r| r.coverage_score)
            .map(|s| format!("{s:.0}"))
            .unwrap_or_else(|| "—".dimmed().to_string());

        table.add_row(vec![
            item.attributes.permalink.clone(),
            type_str,
            truncate(&item.attributes.title, title_max),
            status_str,
            score,
        ]);
    }

    println!("{table}");

    if let Some(meta) = meta {
        println!(
            "{}",
            format!("Showing {} of {} items", items.len(), meta.total).dimmed()
        );
    }
}

fn item_detail(item: &SpecItem) {
    let a = &item.attributes;
    println!("{} {}", a.permalink.bold(), a.title.bold());
    println!("Type: {}", a.type_of);
    println!("Status: {}", a.implementation_status);
    if !a.tags.is_empty() {
        println!("Tags: {}", a.tags.join(", "));
    }
    if let Some(ref desc) = a.description {
        println!("\n{desc}");
    }
    if let Some(ref review) = a.review {
        println!("\n{}", "Review".underline());
        if let Some(score) = review.coverage_score {
            println!("  Score: {score:.0}");
        }
        if let Some(ref verdict) = review.verdict {
            println!("  Verdict: {verdict}");
        }
    }
}

fn item_detail_markdown(item: &SpecItem) {
    let a = &item.attributes;
    println!("## {} — {}\n", a.permalink, a.title);
    println!("- **Type**: {}", a.type_of);
    println!("- **Status**: {}", a.implementation_status);
    if !a.tags.is_empty() {
        println!("- **Tags**: {}", a.tags.join(", "));
    }
    if let Some(ref desc) = a.description {
        println!("\n### Description\n\n{desc}");
    }
    if let Some(ref review) = a.review {
        println!("\n### Review\n");
        if let Some(score) = review.coverage_score {
            println!("- **Score**: {score:.0}");
        }
        if let Some(ref verdict) = review.verdict {
            println!("- **Verdict**: {verdict}");
        }
    }
}

// ---------------------------------------------------------------------------
// Stats / Dashboard
// ---------------------------------------------------------------------------

/// Print the full project dashboard to stdout.
pub fn format_stats_dashboard(
    project_name: &str,
    stats: &funcspec_client::models::ProjectStats,
    usage: Option<&funcspec_client::models::UsageStats>,
) {
    let header = format!("FuncSpec — {project_name}");
    let sep = "─".repeat(header.chars().count().max(36));
    println!("{}", header.bold());
    println!("{}", sep.dimmed());

    // Items
    let total = stats.spec_items.total;
    let functional = stats
        .spec_items
        .by_type
        .get("functional")
        .copied()
        .unwrap_or(0);
    let technical = stats
        .spec_items
        .by_type
        .get("technical")
        .copied()
        .unwrap_or(0);
    println!(
        "{:<12}{} total ({} functional, {} technical)",
        "Items:".cyan().bold(),
        total,
        functional.to_string().blue(),
        technical.to_string().magenta(),
    );

    // Status breakdown
    let implemented = stats
        .spec_items
        .by_implementation
        .get("implemented")
        .copied()
        .unwrap_or(0);
    let in_progress = stats
        .spec_items
        .by_implementation
        .get("in_progress")
        .copied()
        .unwrap_or(0);
    let not_started = stats
        .spec_items
        .by_implementation
        .get("not_started")
        .copied()
        .unwrap_or(0);
    let pct_impl = percent(implemented, total);
    let bar = progress_bar(pct_impl, 10);
    println!(
        "{:<12}{} {} implemented ({:.1}%) │ {} in progress │ {} not started",
        "Status:".cyan().bold(),
        bar,
        implemented.to_string().green(),
        pct_impl,
        in_progress.to_string().yellow(),
        not_started.to_string().dimmed(),
    );

    // Review coverage
    let rev = &stats.reviews;
    let reviewed = rev.tech_reviewed + rev.func_reviewed;
    let pct_rev = percent(reviewed, total);
    let avg = rev
        .avg_tech_score
        .map(|s| format!(" │ avg tech score {s:.1}"))
        .unwrap_or_default();
    println!(
        "{:<12}{} reviewed ({:.1}%){}",
        "Reviews:".cyan().bold(),
        reviewed,
        pct_rev,
        avg,
    );

    // Verdict distribution
    let pass = rev.by_verdict.get("pass").copied().unwrap_or(0);
    let needs_ref = rev.by_verdict.get("needs_refinement").copied().unwrap_or(0);
    let major_gaps = rev.by_verdict.get("major_gaps").copied().unwrap_or(0);
    println!(
        "{:<12}{} pass │ {} needs refinement │ {} major gaps",
        "Verdicts:".cyan().bold(),
        pass.to_string().green(),
        needs_ref.to_string().yellow(),
        major_gaps.to_string().red(),
    );

    // Usage (inline, if caller fetched it)
    if let Some(u) = usage {
        println!(
            "{:<12}{} tokens this month (~${:.2})",
            "Usage:".cyan().bold(),
            format_token_count(u.total_tokens),
            u.estimated_cost,
        );
    }

    // Recent activity summary
    let act = &stats.recent_activity;
    if act.items_updated_24h > 0 || act.reviews_24h > 0 || act.agent_runs_24h > 0 {
        println!(
            "{:<12}{} items updated │ {} reviews │ {} agent runs (24h)",
            "Activity:".cyan().bold(),
            act.items_updated_24h,
            act.reviews_24h,
            act.agent_runs_24h,
        );
    }
}

/// Print the LLM usage summary to stdout.
pub fn format_usage_stats(stats: &funcspec_client::models::UsageStats) {
    let header = format!("Usage — {}", stats.month);
    let sep = "─".repeat(header.chars().count().max(36));
    println!("{}", header.bold());
    println!("{}", sep.dimmed());
    println!(
        "{:<12}{} tokens",
        "Total:".cyan().bold(),
        format_token_count(stats.total_tokens),
    );
    println!("{:<12}~${:.2}", "Cost:".cyan().bold(), stats.estimated_cost);

    if !stats.breakdown_by_operation.is_empty() {
        println!("\n{}", "By operation:".cyan().bold());
        let mut ops: Vec<(&String, &funcspec_client::models::TokenUsage)> =
            stats.breakdown_by_operation.iter().collect();
        ops.sort_by(|a, b| b.1.tokens.cmp(&a.1.tokens));
        for (op, u) in ops {
            println!(
                "  {:<22} {} tokens  ~${:.2}",
                op,
                format_token_count(u.tokens),
                u.cost,
            );
        }
    }
}

// -- helpers -----------------------------------------------------------------

/// Build a Unicode block progress bar of `width` characters representing `pct` (0–100).
pub fn progress_bar(pct: f64, width: usize) -> String {
    let filled = ((pct / 100.0) * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Format a large token count compactly: 45_200 → "45.2k", 1_500_000 → "1.5M".
pub fn format_token_count(n: u32) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Express a UTC timestamp as a human-readable relative time string.
pub fn format_relative_time(dt: &chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let diff = now.signed_duration_since(*dt);
    let secs = diff.num_seconds();
    if secs < 60 {
        return "just now".to_string();
    }
    let mins = diff.num_minutes();
    if mins < 60 {
        return format!("{} minute{} ago", mins, if mins == 1 { "" } else { "s" });
    }
    let hours = diff.num_hours();
    if hours < 24 {
        return format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" });
    }
    let days = diff.num_days();
    if days < 30 {
        return format!("{} day{} ago", days, if days == 1 { "" } else { "s" });
    }
    dt.format("%Y-%m-%d").to_string()
}

fn percent(count: u32, total: u32) -> f64 {
    if total == 0 {
        0.0
    } else {
        (count as f64 / total as f64) * 100.0
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── truncate ────────────────────────────────────────────────────────────

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_exact_length_unchanged() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn truncate_long_string_adds_ellipsis() {
        let result = truncate("hello world", 8);
        assert_eq!(result.chars().count(), 8);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn truncate_handles_unicode() {
        // emoji is 1 char by char count
        let s = "ab🦀cde";
        let result = truncate(s, 4);
        assert_eq!(result.chars().count(), 4);
        assert!(result.ends_with('…'));
    }

    // ── OutputFormat::resolve ───────────────────────────────────────────────

    #[test]
    fn non_auto_formats_resolve_to_themselves() {
        assert_eq!(OutputFormat::Table.resolve(), OutputFormat::Table);
        assert_eq!(OutputFormat::Json.resolve(), OutputFormat::Json);
        assert_eq!(OutputFormat::Csv.resolve(), OutputFormat::Csv);
        assert_eq!(OutputFormat::Minimal.resolve(), OutputFormat::Minimal);
        assert_eq!(OutputFormat::Markdown.resolve(), OutputFormat::Markdown);
        assert_eq!(OutputFormat::Bare.resolve(), OutputFormat::Bare);
    }

    #[test]
    fn auto_resolves_to_table_or_json() {
        let resolved = OutputFormat::Auto.resolve();
        assert!(
            resolved == OutputFormat::Table || resolved == OutputFormat::Json,
            "Auto must resolve to Table (TTY) or Json (pipe), got {resolved:?}"
        );
        assert_ne!(resolved, OutputFormat::Auto);
    }

    // ── terminal_width ──────────────────────────────────────────────────────

    #[test]
    fn terminal_width_is_positive() {
        assert!(terminal_width() > 0);
    }

    // ── format_projects (smoke tests with empty slices) ──────────────────

    #[test]
    fn format_projects_json_empty() {
        let result = format_projects(&[], OutputFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn format_projects_csv_empty() {
        let result = format_projects(&[], OutputFormat::Csv);
        assert!(result.is_ok());
    }

    #[test]
    fn format_projects_minimal_empty() {
        let result = format_projects(&[], OutputFormat::Minimal);
        assert!(result.is_ok());
    }

    #[test]
    fn format_projects_markdown_empty() {
        let result = format_projects(&[], OutputFormat::Markdown);
        assert!(result.is_ok());
    }

    #[test]
    fn format_projects_table_empty() {
        let result = format_projects(&[], OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn format_projects_bare_empty() {
        let result = format_projects(&[], OutputFormat::Bare);
        assert!(result.is_ok());
    }

    // ── format_items (smoke tests with empty slices) ─────────────────────

    #[test]
    fn format_items_json_empty() {
        let result = format_items(&[], None, OutputFormat::Json);
        assert!(result.is_ok());
    }

    #[test]
    fn format_items_csv_empty() {
        let result = format_items(&[], None, OutputFormat::Csv);
        assert!(result.is_ok());
    }

    #[test]
    fn format_items_minimal_empty() {
        let result = format_items(&[], None, OutputFormat::Minimal);
        assert!(result.is_ok());
    }

    #[test]
    fn format_items_markdown_empty() {
        let result = format_items(&[], None, OutputFormat::Markdown);
        assert!(result.is_ok());
    }

    #[test]
    fn format_items_table_empty() {
        let result = format_items(&[], None, OutputFormat::Table);
        assert!(result.is_ok());
    }

    #[test]
    fn format_items_bare_empty() {
        let result = format_items(&[], None, OutputFormat::Bare);
        assert!(result.is_ok());
    }

    // ── JSON output correctness ─────────────────────────────────────────────

    #[test]
    fn format_projects_json_is_valid_json() {
        // Just verify serde_json round-trips cleanly for an empty array
        let empty: Vec<Project> = vec![];
        let json = serde_json::to_string_pretty(&empty).unwrap();
        assert_eq!(json.trim(), "[]");
    }

    #[test]
    fn format_items_json_is_valid_json() {
        let empty: Vec<SpecItem> = vec![];
        let json = serde_json::to_string_pretty(&empty).unwrap();
        assert_eq!(json.trim(), "[]");
    }

    // ── table column width ──────────────────────────────────────────────────

    #[test]
    fn title_truncation_respects_terminal_width() {
        // With a very narrow terminal (60 cols), title_max = max(30, 60-50) = 30
        let title_max = 60usize.saturating_sub(50).max(30);
        let long = "a".repeat(100);
        let result = truncate(&long, title_max);
        assert_eq!(result.chars().count(), title_max);
    }

    #[test]
    fn title_truncation_wide_terminal() {
        // With a wide terminal (200 cols), title_max = 200-50 = 150
        let title_max = 200usize.saturating_sub(50).max(30);
        let short = "hello";
        // Short strings are not truncated
        assert_eq!(truncate(short, title_max), "hello");
    }

    // ── format_diff ─────────────────────────────────────────────────────────

    #[test]
    fn format_diff_does_not_panic() {
        // Ensure the function runs without panicking on arbitrary input
        format_diff("old line\n", "new line\n");
        format_diff("", "");
        format_diff("same\n", "same\n");
    }

    // ── progress_bar ────────────────────────────────────────────────────────

    #[test]
    fn progress_bar_full() {
        let bar = progress_bar(100.0, 10);
        assert_eq!(bar, "██████████");
    }

    #[test]
    fn progress_bar_empty() {
        let bar = progress_bar(0.0, 10);
        assert_eq!(bar, "░░░░░░░░░░");
    }

    #[test]
    fn progress_bar_half() {
        let bar = progress_bar(50.0, 10);
        assert_eq!(bar.chars().count(), 10);
        assert!(bar.starts_with('█'));
        assert!(bar.ends_with('░'));
    }

    #[test]
    fn progress_bar_width_respected() {
        for width in [5, 10, 20] {
            let bar = progress_bar(75.0, width);
            assert_eq!(bar.chars().count(), width);
        }
    }

    // ── format_token_count ──────────────────────────────────────────────────

    #[test]
    fn format_token_count_small() {
        assert_eq!(format_token_count(500), "500");
    }

    #[test]
    fn format_token_count_thousands() {
        assert_eq!(format_token_count(45_200), "45.2k");
    }

    #[test]
    fn format_token_count_millions() {
        assert_eq!(format_token_count(1_500_000), "1.5M");
    }

    #[test]
    fn format_token_count_exact_thousand() {
        assert_eq!(format_token_count(1_000), "1.0k");
    }

    // ── format_relative_time ────────────────────────────────────────────────

    #[test]
    fn format_relative_time_just_now() {
        let dt = chrono::Utc::now();
        assert_eq!(format_relative_time(&dt), "just now");
    }

    #[test]
    fn format_relative_time_minutes() {
        let dt = chrono::Utc::now() - chrono::Duration::minutes(5);
        let s = format_relative_time(&dt);
        assert!(s.contains("minute"), "expected 'minute' in '{s}'");
    }

    #[test]
    fn format_relative_time_hours() {
        let dt = chrono::Utc::now() - chrono::Duration::hours(3);
        let s = format_relative_time(&dt);
        assert!(s.contains("hour"), "expected 'hour' in '{s}'");
    }

    #[test]
    fn format_relative_time_days() {
        let dt = chrono::Utc::now() - chrono::Duration::days(4);
        let s = format_relative_time(&dt);
        assert!(s.contains("day"), "expected 'day' in '{s}'");
    }

    #[test]
    fn format_relative_time_old_date() {
        // More than 30 days ago should return YYYY-MM-DD
        let dt = chrono::Utc::now() - chrono::Duration::days(60);
        let s = format_relative_time(&dt);
        assert!(s.contains('-'), "expected date format in '{s}'");
    }

    // ── format_stats_dashboard smoke test ───────────────────────────────────

    #[test]
    fn format_stats_dashboard_does_not_panic() {
        use funcspec_client::models::*;

        let mut by_type = std::collections::HashMap::new();
        by_type.insert("functional".to_string(), 12u32);
        by_type.insert("technical".to_string(), 30u32);

        let mut by_implementation = std::collections::HashMap::new();
        by_implementation.insert("implemented".to_string(), 28u32);
        by_implementation.insert("in_progress".to_string(), 8u32);
        by_implementation.insert("not_started".to_string(), 6u32);

        let mut by_verdict = std::collections::HashMap::new();
        by_verdict.insert("pass".to_string(), 20u32);
        by_verdict.insert("needs_refinement".to_string(), 12u32);

        let stats = ProjectStats {
            resource_type: Some("project_stats".to_string()),
            spec_items: StatsSpecItems {
                total: 42,
                by_type,
                by_state: Default::default(),
                by_implementation,
            },
            reviews: StatsReviews {
                tech_reviewed: 30,
                tech_unreviewed: 0,
                func_reviewed: 5,
                func_unreviewed: 7,
                avg_tech_score: Some(87.2),
                avg_func_score: None,
                by_verdict,
            },
            coverage: StatsCoverage {
                functional_with_tech: 5,
                functional_without_tech: 7,
            },
            recent_activity: StatsRecentActivity {
                items_updated_24h: 2,
                reviews_24h: 3,
                agent_runs_24h: 0,
            },
        };
        format_stats_dashboard("my-project", &stats, None);
    }

    #[test]
    fn format_usage_stats_does_not_panic() {
        use funcspec_client::models::*;
        let mut breakdown = std::collections::HashMap::new();
        breakdown.insert(
            "review".to_string(),
            TokenUsage {
                tokens: 30_000,
                cost: 0.08,
            },
        );

        let stats = UsageStats {
            month: "2026-03".to_string(),
            total_tokens: 45_200,
            estimated_cost: 0.12,
            breakdown_by_operation: breakdown,
            last_updated: chrono::Utc::now(),
        };
        format_usage_stats(&stats);
    }
}
