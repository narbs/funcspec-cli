use colored::Colorize;
use comfy_table::{ContentArrangement, Table};
use funcspec_client::models::*;

/// Format a list of projects as a table.
pub fn projects_table(projects: &[Project]) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["Slug", "Name", "Updated"]);

    for p in projects {
        table.add_row(vec![
            p.attributes.slug.clone(),
            p.attributes.name.clone(),
            p.attributes.updated_at.format("%Y-%m-%d").to_string(),
        ]);
    }

    println!("{table}");
}

/// Format a list of spec items as a table.
pub fn items_table(items: &[SpecItem], meta: Option<&PaginationMeta>) {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec!["ID", "Type", "Title", "Status", "Score"]);

    for item in items {
        let type_str = match item.attributes.type_of {
            ItemType::Functional => "func".blue().to_string(),
            ItemType::Technical => "tech".magenta().to_string(),
        };

        let status_str = match item.attributes.implementation_status {
            ImplementationStatus::Implemented => "✅ implemented".green().to_string(),
            ImplementationStatus::InProgress => "🟡 in_progress".yellow().to_string(),
            ImplementationStatus::NotStarted => "⚪ not_started".dimmed().to_string(),
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
            truncate(&item.attributes.title, 50),
            status_str,
            score,
        ]);
    }

    println!("{table}");

    if let Some(meta) = meta {
        let showing = items.len();
        println!(
            "{}",
            format!("Showing {showing} of {} items", meta.total).dimmed()
        );
    }
}

/// Print a single item in detail.
pub fn item_detail(item: &SpecItem) {
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

/// Print items as JSON.
pub fn items_json(items: &[SpecItem]) {
    let json = serde_json::to_string_pretty(items).unwrap_or_default();
    println!("{json}");
}

/// Print items as quiet (permalinks only).
pub fn items_quiet(items: &[SpecItem]) {
    for item in items {
        println!("{}", item.attributes.permalink);
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}
