//! Struct-drift contract tests.
//!
//! These decode REAL server-shaped JSON bodies through the client response
//! structs and assert that the fields the CLI depends on survive. They guard
//! the recurring class of bug where a client struct drifts from the server:
//!   - a wrong-shape body makes the ENTIRE parse fail (command returns nothing), or
//!   - a field the CLI relies on silently decodes to null/absent.
//!
//! When the server adds a field, these still pass (forward-compatible, via
//! #[serde(default)]). When a struct is edited so a depended-on field no longer
//! decodes from the documented server shape, the matching assertion fails here
//! in CI instead of silently in production.
//!
//! Bodies below are trimmed captures of actual prod responses for the structs
//! that have drifted historically (AuditRun release_version_id/audit_branch,
//! AuditResult's status_recommendation shape, the flat Job body, AsyncJobRef).

use funcspec_client::models::{AsyncJobRef, AuditResult, AuditRun, Job, JobStatus};

#[test]
fn audit_run_keeps_release_version_and_branch() {
    // The fields that cost a prod goose-chase when the struct lacked them.
    let body = r#"{
        "id": 23,
        "name": "PTUI Full E2E",
        "state": "completed",
        "requested_concurrency": 1,
        "effective_concurrency": 1,
        "filter_criteria": {"item_ids": [1,2,3]},
        "release_version_id": 16,
        "audit_branch": "main",
        "created_by_user_id": 7,
        "started_at": "2026-10-26T00:00:00Z",
        "completed_at": "2026-10-26T00:10:06Z",
        "created_at": "2026-10-25T23:00:00Z",
        "updated_at": "2026-10-26T00:10:06Z",
        "stats": {"total": 33, "done": 33, "failed": 0},
        "items": [],
        "page": 1,
        "total_pages": 1,
        "a_field_the_server_added_later": "ignored"
    }"#;
    let run: AuditRun = serde_json::from_str(body).expect("AuditRun must decode a real body");
    assert_eq!(
        run.release_version_id,
        Some(16),
        "release_version_id dropped"
    );
    assert_eq!(
        run.audit_branch.as_deref(),
        Some("main"),
        "audit_branch dropped"
    );
    assert_eq!(run.stats.total, 33);
    assert_eq!(run.stats.done, 33);
}

#[test]
fn audit_result_matches_server_shape() {
    // The real single-item audit shape (status_recommendation/confidence/...),
    // not the old passed/details shape that failed every single-item audit.
    let body = r#"{
        "id": 99,
        "status_recommendation": "partial",
        "confidence": "medium",
        "files_discovered": 4,
        "coverage_percentage": 42.0,
        "criteria_coverage": [
            {"criterion": "renders preview", "status": "implemented", "evidence": []}
        ],
        "notes": "Some gaps.",
        "created_at": "2026-10-26T00:00:00Z"
    }"#;
    let res: AuditResult = serde_json::from_str(body).expect("AuditResult must decode a real body");
    assert_eq!(res.status_recommendation, "partial");
    assert_eq!(res.confidence, "medium");
    assert_eq!(res.coverage_percentage, 42.0);
    assert_eq!(res.criteria_coverage.len(), 1);
}

#[test]
fn job_body_is_flat_not_jsonapi_wrapped() {
    // batch_jobs#show renders a FLAT object; the Job struct flattens attributes.
    let body = r#"{
        "id": 50,
        "job_type": "func_review",
        "status": "completed",
        "progress": {"item_id": 5, "verdict": "ok"},
        "result": "done",
        "created_at": "2026-10-26T00:00:00Z",
        "updated_at": "2026-10-26T00:01:00Z"
    }"#;
    let job: Job = serde_json::from_str(body).expect("Job must decode the flat server body");
    assert_eq!(job.id, 50);
    assert_eq!(job.attributes.job_type, "func_review");
    assert_eq!(job.attributes.status, JobStatus::Completed);
    // polymorphic progress must not break decode
    assert!(job.attributes.progress.is_some());
}

#[test]
fn async_job_ref_decodes_202_body() {
    // The async POST 202 body: {data:{job_id,status}} -> inner object here.
    let body = r#"{"job_id": 77, "status": "queued"}"#;
    let r: AsyncJobRef = serde_json::from_str(body).expect("AsyncJobRef must decode the 202 ref");
    assert_eq!(r.job_id, Some(77));
    assert_eq!(r.status.as_deref(), Some("queued"));
}
