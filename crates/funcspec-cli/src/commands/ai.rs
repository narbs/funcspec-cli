//! AI operations commands: review, improve, generate, audit.

use std::time::Duration;

use anyhow::{Context, Result};
use colored::Colorize;
use comfy_table::{Attribute, Cell, ContentArrangement, Table};
use funcspec_client::{FuncspecClient, JobStatus, models::*};
use indicatif::{ProgressBar, ProgressStyle};
use rust_i18n::t;

use crate::context::client_and_project;
use crate::output::format_diff;

// ---------------------------------------------------------------------------
// Command definition
// ---------------------------------------------------------------------------

pub enum AiCmd {
    Review { permalink: String },
    ReviewAll,
    Improve { permalink: String, auto_accept: bool },
    Generate { permalink: String },
    Audit { permalink: String },
}

pub fn build_command() -> clap::Command {
    clap::Command::new("ai")
        .about(t!("cmd.ai.about").to_string())
        .arg_required_else_help(true)
        .subcommand(
            clap::Command::new("review")
                .about(t!("cmd.ai.review.about").to_string())
                .arg(clap::Arg::new("permalink").required(true).help(t!("cmd.ai.review.permalink").to_string())),
        )
        .subcommand(
            clap::Command::new("review-all")
                .about(t!("cmd.ai.review_all.about").to_string()),
        )
        .subcommand(
            clap::Command::new("improve")
                .about(t!("cmd.ai.improve.about").to_string())
                .arg(clap::Arg::new("permalink").required(true).help(t!("cmd.ai.improve.permalink").to_string()))
                .arg(clap::Arg::new("auto_accept").long("auto-accept").action(clap::ArgAction::SetTrue).help(t!("cmd.ai.improve.auto_accept").to_string())),
        )
        .subcommand(
            clap::Command::new("generate")
                .about(t!("cmd.ai.generate.about").to_string())
                .arg(clap::Arg::new("permalink").required(true).help(t!("cmd.ai.generate.permalink").to_string())),
        )
        .subcommand(
            clap::Command::new("audit")
                .about(t!("cmd.ai.audit.about").to_string())
                .arg(clap::Arg::new("permalink").required(true).help(t!("cmd.ai.audit.permalink").to_string())),
        )
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub async fn dispatch(matches: &clap::ArgMatches) -> Result<()> {
    let cmd = match matches.subcommand() {
        Some(("review", m)) => AiCmd::Review {
            permalink: m.get_one::<String>("permalink").unwrap().clone(),
        },
        Some(("review-all", _)) => AiCmd::ReviewAll,
        Some(("improve", m)) => AiCmd::Improve {
            permalink: m.get_one::<String>("permalink").unwrap().clone(),
            auto_accept: m.get_flag("auto_accept"),
        },
        Some(("generate", m)) => AiCmd::Generate {
            permalink: m.get_one::<String>("permalink").unwrap().clone(),
        },
        Some(("audit", m)) => AiCmd::Audit {
            permalink: m.get_one::<String>("permalink").unwrap().clone(),
        },
        _ => {
            build_command().print_help().ok();
            return Ok(());
        }
    };
    run(cmd).await
}

pub async fn run(cmd: AiCmd) -> Result<()> {
    match cmd {
        AiCmd::Review { permalink } => handle_review(&permalink).await,
        AiCmd::ReviewAll => handle_review_all().await,
        AiCmd::Improve {
            permalink,
            auto_accept,
        } => handle_improve(&permalink, auto_accept).await,
        AiCmd::Generate { permalink } => handle_generate(&permalink).await,
        AiCmd::Audit { permalink } => handle_audit(&permalink).await,
    }
}

// ---------------------------------------------------------------------------
// Spinner helper
// ---------------------------------------------------------------------------

fn make_spinner(msg: &str) -> ProgressBar {
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

// ---------------------------------------------------------------------------
// Job polling with spinner + Ctrl-C
// ---------------------------------------------------------------------------

async fn poll_job_with_spinner(
    client: &FuncspecClient,
    job_id: u64,
    spinner_msg: &str,
) -> Result<funcspec_client::Job> {
    let pb = make_spinner(spinner_msg);
    let timeout = Duration::from_secs(300);

    let result = tokio::select! {
        result = client.poll_job_until_done(job_id, timeout) => result,
        _ = tokio::signal::ctrl_c() => {
            pb.finish_with_message("Interrupted");
            anyhow::bail!("Interrupted by user (Ctrl-C)");
        }
    };

    let job = result.context("Job polling failed")?;
    match job.attributes.status {
        JobStatus::Completed => pb.finish_with_message("Complete"),
        JobStatus::Failed => {
            let err = job.attributes.result.as_deref().unwrap_or("unknown error");
            pb.finish_with_message(format!("Failed: {err}"));
        }
        _ => pb.finish_with_message("Finished"),
    }
    Ok(job)
}

// ---------------------------------------------------------------------------
// review <permalink>
// ---------------------------------------------------------------------------

async fn handle_review(permalink: &str) -> Result<()> {
    let (client, project_id) = client_and_project().await?;

    let pb = make_spinner(&format!("Fetching {permalink}…"));
    let item = client
        .get_item(project_id, permalink)
        .await
        .with_context(|| format!("Item '{permalink}' not found"))?;
    pb.finish_and_clear();

    let pb2 = make_spinner("Running AI review…");
    let review = client
        .review_item(project_id, item.id)
        .await
        .context("AI review failed")?;
    pb2.finish_and_clear();

    display_review(&item, &review);
    Ok(())
}

// ---------------------------------------------------------------------------
// review-all
// ---------------------------------------------------------------------------

async fn handle_review_all() -> Result<()> {
    let (client, project_id) = client_and_project().await?;

    let pb = make_spinner("Triggering batch review…");
    let job = client
        .review_all(project_id)
        .await
        .context("Failed to start batch review")?;
    pb.finish_and_clear();

    println!("Job {} queued.", job.id);

    let final_job = poll_job_with_spinner(
        &client,
        job.id,
        &format!("Job {} — reviewing all items…", job.id),
    )
    .await?;

    match final_job.attributes.status {
        JobStatus::Completed => {
            println!("{}", "Batch review complete.".green().bold());
            if let Some(result) = &final_job.attributes.result {
                println!("{result}");
            }
        }
        JobStatus::Failed => {
            let msg = final_job
                .attributes
                .result
                .as_deref()
                .unwrap_or("unknown error");
            anyhow::bail!("Batch review job failed: {msg}");
        }
        _ => println!(
            "Job ended with unexpected status: {}",
            final_job.attributes.status
        ),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// improve <permalink>
// ---------------------------------------------------------------------------

async fn handle_improve(permalink: &str, auto_accept: bool) -> Result<()> {
    let (client, project_id) = client_and_project().await?;

    let pb = make_spinner(&format!("Fetching {permalink}…"));
    let item = client
        .get_item(project_id, permalink)
        .await
        .with_context(|| format!("Item '{permalink}' not found"))?;
    pb.finish_and_clear();

    let pb2 = make_spinner("Generating improvement proposal…");
    let proposal = client
        .propose_improvement(project_id, item.id)
        .await
        .context("Failed to generate proposal")?;
    pb2.finish_and_clear();

    // Show diff
    let original = proposal
        .attributes
        .original_description
        .as_deref()
        .unwrap_or("");
    let proposed = proposal
        .attributes
        .proposed_description
        .as_deref()
        .unwrap_or("");

    println!("\n{}", "=== Proposed Improvement ===".bold());
    println!(
        "{} {} — {}",
        "Item:".bold(),
        permalink.cyan(),
        item.attributes.title
    );
    if let Some(ref rationale) = proposal.attributes.rationale {
        println!("{} {}", "Rationale:".bold(), rationale);
    }
    println!();
    println!("{}", "--- current".red());
    println!("{}", "+++ proposed".green());
    format_diff(original, proposed);
    println!();

    // Accept / reject
    let accepted = if auto_accept {
        println!("{}", "Auto-accepting proposal (--auto-accept).".yellow());
        true
    } else {
        dialoguer::Confirm::new()
            .with_prompt("Accept this improvement?")
            .default(false)
            .interact()
            .context("Prompt error")?
    };

    if accepted {
        let pb3 = make_spinner("Accepting proposal…");
        client
            .accept_proposal(project_id, item.id)
            .await
            .context("Failed to accept proposal")?;
        pb3.finish_and_clear();
        println!("{}", "Proposal accepted.".green().bold());
    } else {
        let pb3 = make_spinner("Rejecting proposal…");
        client
            .reject_proposal(project_id, item.id)
            .await
            .context("Failed to reject proposal")?;
        pb3.finish_and_clear();
        println!("{}", "Proposal rejected.".yellow());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// generate <permalink>
// ---------------------------------------------------------------------------

async fn handle_generate(permalink: &str) -> Result<()> {
    let (client, project_id) = client_and_project().await?;

    let pb = make_spinner(&format!("Fetching {permalink}…"));
    let item = client
        .get_item(project_id, permalink)
        .await
        .with_context(|| format!("Item '{permalink}' not found"))?;
    pb.finish_and_clear();

    let pb2 = make_spinner("Generating technical spec proposals…");
    let tech_proposals = client
        .generate_tech(project_id, item.id)
        .await
        .context("Failed to generate tech specs")?;
    pb2.finish_and_clear();

    println!("\n{}", "=== Generated Tech Spec Proposals ===".bold());
    println!(
        "{} {} — {}",
        "From:".bold(),
        permalink.cyan(),
        item.attributes.title
    );

    if tech_proposals.proposals.is_empty() {
        println!("{}", "No proposals generated.".yellow());
        return Ok(());
    }

    println!(
        "{} proposal(s) generated.\n",
        tech_proposals.proposals.len()
    );

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        Cell::new("#").add_attribute(Attribute::Bold),
        Cell::new("Title").add_attribute(Attribute::Bold),
        Cell::new("Type").add_attribute(Attribute::Bold),
        Cell::new("Rationale").add_attribute(Attribute::Bold),
    ]);

    for (i, proposal) in tech_proposals.proposals.iter().enumerate() {
        table.add_row(vec![
            Cell::new(i + 1),
            Cell::new(&proposal.title),
            Cell::new(&proposal.type_of),
            Cell::new(proposal.rationale.as_deref().unwrap_or("—")),
        ]);
    }

    println!("{table}");
    Ok(())
}

// ---------------------------------------------------------------------------
// audit <permalink>
// ---------------------------------------------------------------------------

async fn handle_audit(permalink: &str) -> Result<()> {
    let (client, project_id) = client_and_project().await?;

    let pb = make_spinner(&format!("Fetching {permalink}…"));
    let item = client
        .get_item(project_id, permalink)
        .await
        .with_context(|| format!("Item '{permalink}' not found"))?;
    pb.finish_and_clear();

    let pb2 = make_spinner("Running code audit…");
    let audit = client
        .audit_item(project_id, item.id)
        .await
        .context("Code audit failed")?;
    pb2.finish_and_clear();

    display_audit(&item, &audit);
    Ok(())
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

fn display_review(item: &SpecItem, review: &Review) {
    let attrs = &review.attributes;
    let score = attrs.coverage_score.unwrap_or(0.0);
    let verdict = attrs.verdict.as_deref().unwrap_or("—");

    println!("\n{}", "=== AI Review Result ===".bold());
    println!(
        "{} {} — {}",
        "Item:".bold(),
        item.attributes.permalink.cyan(),
        item.attributes.title
    );

    if let Some(ref title) = attrs.tech_item_title {
        println!("{} {}", "Tech spec:".bold(), title);
    }

    let score_str = format!("{score:.1}");
    let score_colored = if score >= 80.0 {
        score_str.green()
    } else if score >= 50.0 {
        score_str.yellow()
    } else {
        score_str.red()
    };
    println!("{} {}%", "Score:".bold(), score_colored);
    if let Some(collective) = attrs.collective_coverage_score {
        println!("{} {:.1}%", "Collective score:".bold(), collective);
    }
    println!("{} {}", "Verdict:".bold(), verdict);

    if !attrs.coverage_map.is_empty() {
        println!("\n{}", "Coverage:".bold());
        for (req, entry) in &attrs.coverage_map {
            let status = entry.get("status").and_then(|v| v.as_str()).unwrap_or("?");
            println!(
                "  {} {req}",
                if status == "covered" {
                    "✓".green()
                } else {
                    "✗".red()
                }
            );
        }
    }

    if !attrs.gaps.is_empty() {
        println!("\n{}", "Gaps Found:".bold());
        for gap in &attrs.gaps {
            println!("  {} {gap}", "!".red());
        }
    }

    if !attrs.risks.is_empty() {
        println!("\n{}", "Risks:".bold());
        for r in &attrs.risks {
            println!("  {} {r}", "⚠".yellow());
        }
    }

    if !attrs.suggestions.is_empty() {
        println!("\n{}", "Suggestions:".bold());
        for s in &attrs.suggestions {
            println!("  {} {s}", "→".blue());
        }
    }

    println!();
}

fn display_audit(item: &SpecItem, audit: &AuditResult) {
    let attrs = &audit.attributes;

    println!("\n{}", "=== Code Audit Result ===".bold());
    println!(
        "{} {} — {}",
        "Item:".bold(),
        item.attributes.permalink.cyan(),
        item.attributes.title
    );
    println!("{} {}", "Audit Type:".bold(), attrs.audit_type);

    let status_str = if attrs.passed {
        "PASSED".green().bold()
    } else {
        "FAILED".red().bold()
    };
    println!("{} {status_str}", "Result:".bold());

    if !attrs.details.is_empty() {
        println!("\n{}", "Details:".bold());
        println!("{}", attrs.details);
    }

    println!();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_cmd_review_parses() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["ai", "review", "F-5"]).unwrap();
        let sub = m.subcommand_matches("review").unwrap();
        assert_eq!(sub.get_one::<String>("permalink").unwrap(), "F-5");
    }

    #[test]
    fn ai_cmd_review_requires_permalink() {
        let cmd = build_command();
        assert!(cmd.try_get_matches_from(["ai", "review"]).is_err());
    }

    #[test]
    fn ai_cmd_review_all_parses() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["ai", "review-all"]).unwrap();
        assert!(m.subcommand_matches("review-all").is_some());
    }

    #[test]
    fn ai_cmd_improve_parses_with_auto_accept() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["ai", "improve", "--auto-accept", "F-10"])
            .unwrap();
        let sub = m.subcommand_matches("improve").unwrap();
        assert_eq!(sub.get_one::<String>("permalink").unwrap(), "F-10");
        assert!(sub.get_flag("auto_accept"));
    }

    #[test]
    fn ai_cmd_improve_auto_accept_defaults_false() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["ai", "improve", "F-10"])
            .unwrap();
        let sub = m.subcommand_matches("improve").unwrap();
        assert!(!sub.get_flag("auto_accept"));
    }

    #[test]
    fn ai_cmd_generate_parses() {
        let cmd = build_command();
        let m = cmd
            .try_get_matches_from(["ai", "generate", "F-100"])
            .unwrap();
        let sub = m.subcommand_matches("generate").unwrap();
        assert_eq!(sub.get_one::<String>("permalink").unwrap(), "F-100");
    }

    #[test]
    fn ai_cmd_audit_parses() {
        let cmd = build_command();
        let m = cmd.try_get_matches_from(["ai", "audit", "F-50"]).unwrap();
        let sub = m.subcommand_matches("audit").unwrap();
        assert_eq!(sub.get_one::<String>("permalink").unwrap(), "F-50");
    }

    // display helpers run without panicking
    #[test]
    fn display_review_does_not_panic() {
        use chrono::Utc;
        use funcspec_client::models::*;

        let item = SpecItem {
            id: 1,
            resource_type: "spec_item".into(),
            attributes: SpecItemAttributes {
                title: "Test item".into(),
                description: None,
                type_of: ItemType::Functional,
                state: "active".into(),
                implementation_status: ImplementationStatus::NotStarted,
                permalink: "F-1".into(),
                url: "https://funcspec.net/items/1".into(),
                version: 1,
                priority: None,
                position: None,
                tags: vec![],
                parent_id: None,
                project_id: 1,
                review: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        };

        let review = Review {
            id: Some(10),
            resource_type: "review".into(),
            attributes: ReviewAttributes {
                coverage_score: Some(85.0),
                collective_coverage_score: None,
                verdict: Some("pass".into()),
                tech_item_id: Some(1),
                tech_item_title: Some("JWT service".into()),
                func_item_ids: vec![1],
                functional_requirements_parsed: vec![],
                coverage_map: {
                    let mut m = std::collections::HashMap::new();
                    m.insert(
                        "Auth flow".into(),
                        serde_json::json!({"status": "covered", "covered_by": "JWT service", "notes": ""}),
                    );
                    m
                },
                gaps: vec!["Missing error handling".into()],
                suggestions: vec!["Add retry logic".into()],
                risks: vec!["Token race condition".into()],
                reviewed_at: Some(Utc::now()),
            },
        };

        // Should not panic
        display_review(&item, &review);
    }

    #[test]
    fn display_audit_does_not_panic() {
        use chrono::Utc;
        use funcspec_client::models::*;

        let item = SpecItem {
            id: 2,
            resource_type: "spec_item".into(),
            attributes: SpecItemAttributes {
                title: "Auth item".into(),
                description: None,
                type_of: ItemType::Technical,
                state: "active".into(),
                implementation_status: ImplementationStatus::Implemented,
                permalink: "T-2".into(),
                url: "https://funcspec.net/items/2".into(),
                version: 1,
                priority: None,
                position: None,
                tags: vec![],
                parent_id: None,
                project_id: 1,
                review: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        };

        let audit = AuditResult {
            id: 5,
            resource_type: "audit_result".into(),
            attributes: AuditResultAttributes {
                spec_item_id: 2,
                audit_type: "coverage".into(),
                passed: false,
                details: "Missing test coverage for error paths".into(),
                created_at: Utc::now(),
            },
        };

        display_audit(&item, &audit);
    }
}
