//! AI operations commands: review, improve, generate, audit.

use std::time::Duration;

use anyhow::{Context, Result};
use clap::Subcommand;
use colored::Colorize;
use comfy_table::{Attribute, Cell, ContentArrangement, Table};
use funcspec_client::{FuncspecClient, JobStatus, models::*};
use indicatif::{ProgressBar, ProgressStyle};

use crate::context::client_and_project;
use crate::output::format_diff;

// ---------------------------------------------------------------------------
// Command definition
// ---------------------------------------------------------------------------

#[derive(Subcommand)]
pub enum AiCmd {
    /// Trigger AI review of a single spec item; shows score, verdict, coverage map, and gaps
    Review {
        /// Item permalink (e.g. F-5)
        permalink: String,
    },

    /// Trigger batch AI review of all items in the project (async, polls until complete)
    ReviewAll,

    /// Propose an AI-generated improvement for a spec item; shows diff and prompts accept/reject
    Improve {
        /// Item permalink (e.g. F-5)
        permalink: String,

        /// Auto-accept the proposal without interactive prompting
        #[arg(long)]
        auto_accept: bool,
    },

    /// Generate technical spec proposals from a functional item
    Generate {
        /// Item permalink (e.g. F-5)
        permalink: String,
    },

    /// Run a code audit on a spec item and show coverage and results
    Audit {
        /// Item permalink (e.g. F-5)
        permalink: String,
    },
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

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
    println!("{} {}", "Reviewer:".bold(), attrs.reviewer);

    let score_str = format!("{score:.1}");
    let score_colored = if score >= 80.0 {
        score_str.green()
    } else if score >= 50.0 {
        score_str.yellow()
    } else {
        score_str.red()
    };
    println!("{} {}%", "Score:".bold(), score_colored);
    println!("{} {}", "Verdict:".bold(), verdict);
    println!("{} {}", "Status:".bold(), attrs.status);

    if !attrs.coverage_map.is_empty() {
        println!("\n{}", "Coverage Map:".bold());
        for area in &attrs.coverage_map {
            println!("  {} {area}", "✓".green());
        }
    }

    if !attrs.gaps.is_empty() {
        println!("\n{}", "Gaps Found:".bold());
        for gap in &attrs.gaps {
            println!("  {} {gap}", "!".red());
        }
    }

    if !attrs.suggestions.is_empty() {
        println!("\n{}", "Suggestions:".bold());
        for s in &attrs.suggestions {
            println!("  {} {s}", "→".blue());
        }
    }

    if let Some(ref comment) = attrs.comment {
        println!("\n{} {comment}", "Comment:".bold());
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

    // Verify AiCmd subcommands parse correctly via clap
    #[test]
    fn ai_cmd_review_parses() {
        use clap::Parser;

        #[derive(Parser)]
        struct TestCli {
            #[command(subcommand)]
            cmd: AiCmd,
        }

        let cli = TestCli::try_parse_from(["test", "review", "F-5"]).unwrap();
        match cli.cmd {
            AiCmd::Review { permalink } => assert_eq!(permalink, "F-5"),
            _ => panic!("expected Review"),
        }
    }

    #[test]
    fn ai_cmd_review_requires_permalink() {
        use clap::Parser;

        #[derive(Parser)]
        struct TestCli {
            #[command(subcommand)]
            cmd: AiCmd,
        }

        assert!(TestCli::try_parse_from(["test", "review"]).is_err());
    }

    #[test]
    fn ai_cmd_review_all_parses() {
        use clap::Parser;

        #[derive(Parser)]
        struct TestCli {
            #[command(subcommand)]
            cmd: AiCmd,
        }

        let cli = TestCli::try_parse_from(["test", "review-all"]).unwrap();
        assert!(matches!(cli.cmd, AiCmd::ReviewAll));
    }

    #[test]
    fn ai_cmd_improve_parses_with_auto_accept() {
        use clap::Parser;

        #[derive(Parser)]
        struct TestCli {
            #[command(subcommand)]
            cmd: AiCmd,
        }

        let cli = TestCli::try_parse_from(["test", "improve", "--auto-accept", "F-10"]).unwrap();
        match cli.cmd {
            AiCmd::Improve {
                permalink,
                auto_accept,
            } => {
                assert_eq!(permalink, "F-10");
                assert!(auto_accept);
            }
            _ => panic!("expected Improve"),
        }
    }

    #[test]
    fn ai_cmd_improve_auto_accept_defaults_false() {
        use clap::Parser;

        #[derive(Parser)]
        struct TestCli {
            #[command(subcommand)]
            cmd: AiCmd,
        }

        let cli = TestCli::try_parse_from(["test", "improve", "F-10"]).unwrap();
        match cli.cmd {
            AiCmd::Improve { auto_accept, .. } => assert!(!auto_accept),
            _ => panic!("expected Improve"),
        }
    }

    #[test]
    fn ai_cmd_generate_parses() {
        use clap::Parser;

        #[derive(Parser)]
        struct TestCli {
            #[command(subcommand)]
            cmd: AiCmd,
        }

        let cli = TestCli::try_parse_from(["test", "generate", "F-100"]).unwrap();
        match cli.cmd {
            AiCmd::Generate { permalink } => assert_eq!(permalink, "F-100"),
            _ => panic!("expected Generate"),
        }
    }

    #[test]
    fn ai_cmd_audit_parses() {
        use clap::Parser;

        #[derive(Parser)]
        struct TestCli {
            #[command(subcommand)]
            cmd: AiCmd,
        }

        let cli = TestCli::try_parse_from(["test", "audit", "F-50"]).unwrap();
        match cli.cmd {
            AiCmd::Audit { permalink } => assert_eq!(permalink, "F-50"),
            _ => panic!("expected Audit"),
        }
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
            id: 10,
            resource_type: "review".into(),
            attributes: ReviewAttributes {
                spec_item_id: 1,
                reviewer: "ai".into(),
                status: ReviewStatus::Approved,
                comment: Some("Looks good".into()),
                coverage_score: Some(85.0),
                verdict: Some("pass".into()),
                coverage_map: vec!["Auth flow".into()],
                gaps: vec!["Missing error handling".into()],
                suggestions: vec!["Add retry logic".into()],
                created_at: Utc::now(),
                updated_at: Utc::now(),
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
