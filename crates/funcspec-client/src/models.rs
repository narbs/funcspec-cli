use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// API envelope
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiListResponse<T> {
    pub data: Vec<T>,
    pub meta: Option<PaginationMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub page: u32,
    pub per: u32,
    pub total: u32,
    #[serde(default)]
    pub total_pages: u32,
}

// ---------------------------------------------------------------------------
// Auth / User
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: String,
    pub org_id: String,
    pub org_name: String,
}

// ---------------------------------------------------------------------------
// Project
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: u64,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub attributes: ProjectAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAttributes {
    pub name: String,
    pub description: Option<String>,
    pub slug: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Spec Item
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecItem {
    pub id: u64,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub attributes: SpecItemAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecItemAttributes {
    pub title: String,
    pub description: Option<String>,
    pub type_of: ItemType,
    pub state: String,
    pub implementation_status: ImplementationStatus,
    pub permalink: String,
    pub url: String,
    pub version: u32,
    pub priority: Option<String>,
    pub position: Option<i32>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub parent_id: Option<u64>,
    pub project_id: u64,
    pub review: Option<ReviewSummary>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    Functional,
    Technical,
}

impl std::fmt::Display for ItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemType::Functional => write!(f, "functional"),
            ItemType::Technical => write!(f, "technical"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ImplementationStatus {
    NotStarted,
    InProgress,
    Implemented,
}

impl std::fmt::Display for ImplementationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImplementationStatus::NotStarted => write!(f, "not_started"),
            ImplementationStatus::InProgress => write!(f, "in_progress"),
            ImplementationStatus::Implemented => write!(f, "implemented"),
        }
    }
}

// ---------------------------------------------------------------------------
// Review
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewSummary {
    pub coverage_score: Option<f64>,
    pub verdict: Option<String>,
    pub review_type: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub fresh: Option<bool>,
    pub coverage_map: Option<serde_json::Value>,
    pub gaps: Option<Vec<String>>,
    pub suggestions: Option<Vec<String>>,
}

/// Returned by `POST /projects/:id/work_package/:id/review`.
/// The API does not include a top-level `id` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub attributes: ReviewAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewAttributes {
    #[serde(default)]
    pub coverage_score: Option<f64>,
    #[serde(default)]
    pub collective_coverage_score: Option<f64>,
    #[serde(default)]
    pub verdict: Option<String>,
    #[serde(default)]
    pub tech_item_id: Option<u64>,
    #[serde(default)]
    pub tech_item_title: Option<String>,
    #[serde(default)]
    pub func_item_ids: Vec<u64>,
    #[serde(default)]
    pub functional_requirements_parsed: Vec<String>,
    /// Map of requirement → coverage entry (status, covered_by, notes).
    #[serde(default)]
    pub coverage_map: std::collections::HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub gaps: Vec<String>,
    #[serde(default)]
    pub suggestions: Vec<String>,
    #[serde(default)]
    pub risks: Vec<String>,
    #[serde(default)]
    pub reviewed_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// AI — Proposal
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub attributes: ProposalAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalAttributes {
    pub spec_item_id: u64,
    pub original_description: Option<String>,
    pub proposed_description: Option<String>,
    pub rationale: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// AI — Tech Proposals
// ---------------------------------------------------------------------------

/// A single proposed technical spec item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechProposal {
    pub title: String,
    pub description: Option<String>,
    pub type_of: String,
    pub rationale: Option<String>,
}

/// Collection of tech spec proposals generated from a functional item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechProposals {
    pub functional_item_id: u64,
    pub functional_item_permalink: String,
    #[serde(default)]
    pub proposals: Vec<TechProposal>,
}

// ---------------------------------------------------------------------------
// Audit
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    pub id: u64,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub attributes: AuditResultAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResultAttributes {
    pub spec_item_id: u64,
    pub audit_type: String,
    pub passed: bool,
    pub details: String,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Snapshot
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: u64,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub attributes: SnapshotAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotAttributes {
    #[serde(default)]
    pub project_id: Option<u64>,
    pub name: String,
    pub description: Option<String>,
    /// Full spec items — present in single-snapshot responses, may be absent in list responses.
    #[serde(default)]
    pub spec_items: Vec<SpecItem>,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Job
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: u64,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub attributes: JobAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobAttributes {
    pub job_type: String,
    pub status: JobStatus,
    pub progress: Option<f32>,
    pub result: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "pending"),
            JobStatus::Running => write!(f, "running"),
            JobStatus::Completed => write!(f, "completed"),
            JobStatus::Failed => write!(f, "failed"),
        }
    }
}

// ---------------------------------------------------------------------------
// Usage Log
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLog {
    pub id: u64,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub attributes: UsageLogAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLogAttributes {
    pub user_id: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Request params
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Serialize)]
pub struct CreateProjectParams {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct UpdateProjectParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct CreateItemParams {
    pub title: String,
    pub type_of: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct UpdateItemParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implementation_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<String>,
}

#[derive(Debug, Default, Serialize)]
pub struct CreateReviewParams {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

// ---------------------------------------------------------------------------
// Query filters
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct ItemFilter {
    pub type_of: Option<ItemType>,
    pub status: Option<ImplementationStatus>,
    pub tag: Option<String>,
    pub q: Option<String>,
    pub has_review: Option<bool>,
    pub review_verdict: Option<String>,
    pub parent_id: Option<u64>,
    pub sort: Option<String>,
    pub page: Option<u32>,
    pub per: Option<u32>,
}

impl ItemFilter {
    pub fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        if let Some(ref t) = self.type_of {
            pairs.push(("type_of".into(), t.to_string()));
        }
        if let Some(ref s) = self.status {
            pairs.push(("implementation_status".into(), s.to_string()));
        }
        if let Some(ref tag) = self.tag {
            pairs.push(("tag".into(), tag.clone()));
        }
        if let Some(ref q) = self.q {
            pairs.push(("q".into(), q.clone()));
        }
        if let Some(true) = self.has_review {
            pairs.push(("has_review".into(), "true".into()));
        }
        if let Some(ref v) = self.review_verdict {
            pairs.push(("review_verdict".into(), v.clone()));
        }
        if let Some(id) = self.parent_id {
            pairs.push(("parent_id".into(), id.to_string()));
        }
        if let Some(ref s) = self.sort {
            pairs.push(("sort".into(), s.clone()));
        }
        if let Some(p) = self.page {
            pairs.push(("page".into(), p.to_string()));
        }
        if let Some(p) = self.per {
            pairs.push(("per".into(), p.to_string()));
        }
        pairs
    }
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

/// Returned by `GET /projects/:id/stats` (inside `{"data": {...}}`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStats {
    #[serde(rename = "type", default)]
    pub resource_type: Option<String>,
    pub spec_items: StatsSpecItems,
    pub reviews: StatsReviews,
    pub coverage: StatsCoverage,
    pub recent_activity: StatsRecentActivity,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatsSpecItems {
    #[serde(default)]
    pub total: u32,
    #[serde(default)]
    pub by_type: std::collections::HashMap<String, u32>,
    #[serde(default)]
    pub by_state: std::collections::HashMap<String, u32>,
    #[serde(default)]
    pub by_implementation: std::collections::HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatsReviews {
    #[serde(default)]
    pub tech_reviewed: u32,
    #[serde(default)]
    pub tech_unreviewed: u32,
    #[serde(default)]
    pub func_reviewed: u32,
    #[serde(default)]
    pub func_unreviewed: u32,
    #[serde(default)]
    pub avg_tech_score: Option<f64>,
    #[serde(default)]
    pub avg_func_score: Option<f64>,
    #[serde(default)]
    pub by_verdict: std::collections::HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatsCoverage {
    #[serde(default)]
    pub functional_with_tech: u32,
    #[serde(default)]
    pub functional_without_tech: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StatsRecentActivity {
    #[serde(default)]
    pub items_updated_24h: u32,
    #[serde(default)]
    pub reviews_24h: u32,
    #[serde(default)]
    pub agent_runs_24h: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub month: String,
    pub total_tokens: u32,
    pub estimated_cost: f64,
    pub breakdown_by_operation: std::collections::HashMap<String, TokenUsage>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub tokens: u32,
    pub cost: f64,
}

// ---------------------------------------------------------------------------
// Usage filter
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct UsageFilter {
    pub month: Option<String>,
    pub page: Option<u32>,
    pub per: Option<u32>,
}

impl UsageFilter {
    pub fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        if let Some(ref m) = self.month {
            pairs.push(("month".into(), m.clone()));
        }
        if let Some(p) = self.page {
            pairs.push(("page".into(), p.to_string()));
        }
        if let Some(p) = self.per {
            pairs.push(("per".into(), p.to_string()));
        }
        pairs
    }
}

// ---------------------------------------------------------------------------
// Snapshot diff
// ---------------------------------------------------------------------------

/// A single modified item: the state before and after the snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiffItem {
    pub before: SpecItem,
    pub after: SpecItem,
}

/// Diff between a snapshot and the current project state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub snapshot_id: u64,
    pub added: Vec<SpecItem>,
    pub removed: Vec<SpecItem>,
    pub modified: Vec<SnapshotDiffItem>,
}

// ---------------------------------------------------------------------------
// Request params — Snapshot
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct CreateSnapshotParams {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_type_display() {
        assert_eq!(ItemType::Functional.to_string(), "functional");
        assert_eq!(ItemType::Technical.to_string(), "technical");
    }

    #[test]
    fn item_type_serde_roundtrip() {
        let json = r#""functional""#;
        let t: ItemType = serde_json::from_str(json).unwrap();
        assert_eq!(t, ItemType::Functional);
        assert_eq!(serde_json::to_string(&t).unwrap(), json);
    }

    #[test]
    fn impl_status_display() {
        assert_eq!(ImplementationStatus::NotStarted.to_string(), "not_started");
        assert_eq!(ImplementationStatus::InProgress.to_string(), "in_progress");
        assert_eq!(ImplementationStatus::Implemented.to_string(), "implemented");
    }

    #[test]
    fn impl_status_serde_roundtrip() {
        let json = r#""in_progress""#;
        let s: ImplementationStatus = serde_json::from_str(json).unwrap();
        assert_eq!(s, ImplementationStatus::InProgress);
    }

    #[test]
    fn job_status_display() {
        assert_eq!(JobStatus::Pending.to_string(), "pending");
        assert_eq!(JobStatus::Completed.to_string(), "completed");
        assert_eq!(JobStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn project_deserialize() {
        let json = r#"{
            "id": 42,
            "type": "project",
            "attributes": {
                "name": "My Project",
                "description": "desc",
                "slug": "my-project",
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-06-01T00:00:00Z"
            }
        }"#;
        let p: Project = serde_json::from_str(json).unwrap();
        assert_eq!(p.id, 42);
        assert_eq!(p.attributes.name, "My Project");
        assert_eq!(p.attributes.slug, "my-project");
    }

    #[test]
    fn spec_item_deserialize_minimal() {
        let json = r#"{
            "id": 1,
            "type": "spec_item",
            "attributes": {
                "title": "Feature X",
                "description": null,
                "type_of": "functional",
                "state": "active",
                "implementation_status": "not_started",
                "permalink": "F-1",
                "url": "https://funcspec.net/items/1",
                "version": 1,
                "priority": null,
                "position": null,
                "tags": [],
                "parent_id": null,
                "project_id": 1,
                "review": null,
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-01-01T00:00:00Z"
            }
        }"#;
        let item: SpecItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.id, 1);
        assert_eq!(item.attributes.permalink, "F-1");
        assert_eq!(item.attributes.type_of, ItemType::Functional);
        assert_eq!(
            item.attributes.implementation_status,
            ImplementationStatus::NotStarted
        );
    }

    #[test]
    fn spec_item_with_review_deserialize() {
        let json = r#"{
            "id": 5,
            "type": "spec_item",
            "attributes": {
                "title": "Auth flow",
                "description": "Login/logout",
                "type_of": "technical",
                "state": "active",
                "implementation_status": "implemented",
                "permalink": "T-5",
                "url": "https://funcspec.net/items/5",
                "version": 2,
                "priority": "high",
                "position": 1,
                "tags": ["auth", "backend"],
                "parent_id": null,
                "project_id": 1,
                "review": {
                    "id": 10,
                    "coverage_score": 85.5,
                    "verdict": "looks good",
                    "updated_at": "2024-06-01T00:00:00Z"
                },
                "created_at": "2024-01-01T00:00:00Z",
                "updated_at": "2024-06-01T00:00:00Z"
            }
        }"#;
        let item: SpecItem = serde_json::from_str(json).unwrap();
        let review = item.attributes.review.unwrap();
        assert_eq!(review.coverage_score, Some(85.5));
        assert_eq!(review.verdict.as_deref(), Some("looks good"));
    }

    #[test]
    fn api_list_response_deserialize() {
        let json = r#"{
            "data": [],
            "meta": {"page": 1, "per": 25, "total": 0, "total_pages": 0}
        }"#;
        let resp: ApiListResponse<Project> = serde_json::from_str(json).unwrap();
        assert!(resp.data.is_empty());
        let meta = resp.meta.unwrap();
        assert_eq!(meta.page, 1);
        assert_eq!(meta.total, 0);
    }

    #[test]
    fn api_list_response_no_meta() {
        let json = r#"{"data": [], "meta": null}"#;
        let resp: ApiListResponse<Project> = serde_json::from_str(json).unwrap();
        assert!(resp.meta.is_none());
    }

    #[test]
    fn item_filter_to_query_pairs_empty() {
        let filter = ItemFilter::default();
        assert!(filter.to_query_pairs().is_empty());
    }

    #[test]
    fn item_filter_sort_param() {
        let filter = ItemFilter {
            sort: Some("score".into()),
            ..Default::default()
        };
        let pairs = filter.to_query_pairs();
        assert!(pairs.iter().any(|(k, v)| k == "sort" && v == "score"));
    }

    #[test]
    fn project_stats_deserialize() {
        let json = r#"{
            "type": "project_stats",
            "spec_items": {
                "total": 42,
                "by_type": {"functional": 12, "technical": 30},
                "by_state": {"inbox": 42},
                "by_implementation": {"implemented": 28, "in_progress": 8, "not_started": 6}
            },
            "reviews": {
                "tech_reviewed": 30,
                "tech_unreviewed": 0,
                "func_reviewed": 5,
                "func_unreviewed": 7,
                "avg_tech_score": 87.2,
                "avg_func_score": null,
                "by_verdict": {"pass": 20, "needs_refinement": 12, "major_gaps": 3}
            },
            "coverage": {
                "functional_with_tech": 5,
                "functional_without_tech": 7
            },
            "recent_activity": {
                "items_updated_24h": 2,
                "reviews_24h": 3,
                "agent_runs_24h": 0
            }
        }"#;
        let s: ProjectStats = serde_json::from_str(json).unwrap();
        assert_eq!(s.spec_items.total, 42);
        assert_eq!(s.spec_items.by_type.get("functional"), Some(&12u32));
        assert_eq!(s.spec_items.by_type.get("technical"), Some(&30u32));
        assert_eq!(s.spec_items.by_implementation.get("implemented"), Some(&28u32));
        assert_eq!(s.reviews.tech_reviewed, 30);
        assert_eq!(s.reviews.avg_tech_score, Some(87.2));
        assert_eq!(s.reviews.avg_func_score, None);
        assert_eq!(s.reviews.by_verdict.get("pass"), Some(&20u32));
        assert_eq!(s.reviews.by_verdict.get("needs_refinement"), Some(&12u32));
        assert_eq!(s.coverage.functional_with_tech, 5);
        assert_eq!(s.recent_activity.items_updated_24h, 2);
        assert_eq!(s.recent_activity.reviews_24h, 3);
    }

    #[test]
    fn usage_stats_deserialize() {
        let json = r#"{
            "month": "2026-03",
            "total_tokens": 45200,
            "estimated_cost": 0.12,
            "breakdown_by_operation": {
                "review": {"tokens": 30000, "cost": 0.08},
                "analysis": {"tokens": 15200, "cost": 0.04}
            },
            "last_updated": "2026-03-24T00:00:00Z"
        }"#;
        let s: UsageStats = serde_json::from_str(json).unwrap();
        assert_eq!(s.month, "2026-03");
        assert_eq!(s.total_tokens, 45200);
        assert!((s.estimated_cost - 0.12).abs() < 1e-9);
        assert_eq!(
            s.breakdown_by_operation.get("review").map(|u| u.tokens),
            Some(30000)
        );
    }

    #[test]
    fn review_attributes_with_ai_fields_deserialize() {
        let json = r#"{
            "data": {
                "type": "review",
                "attributes": {
                    "coverage_score": 87.5,
                    "collective_coverage_score": null,
                    "verdict": "pass",
                    "tech_item_id": 10,
                    "tech_item_title": "JWT service",
                    "func_item_ids": [5],
                    "functional_requirements_parsed": ["Login", "logout", "reset"],
                    "coverage_map": {
                        "Authentication flow": {"status": "covered", "covered_by": "JWT service", "notes": ""}
                    },
                    "gaps": ["Missing edge case for expired tokens"],
                    "suggestions": ["Add retry logic"],
                    "risks": ["Token expiry race condition"],
                    "reviewed_at": "2026-01-01T00:00:00Z"
                }
            }
        }"#;
        let resp: ApiResponse<Review> = serde_json::from_str(json).unwrap();
        let attrs = &resp.data.attributes;
        assert_eq!(attrs.coverage_score, Some(87.5));
        assert_eq!(attrs.coverage_map.len(), 1);
        assert_eq!(attrs.gaps.len(), 1);
        assert_eq!(attrs.suggestions.len(), 1);
        assert_eq!(attrs.risks.len(), 1);
        assert_eq!(attrs.gaps[0], "Missing edge case for expired tokens");
        assert_eq!(attrs.tech_item_title.as_deref(), Some("JWT service"));
    }

    #[test]
    fn review_attributes_missing_ai_fields_defaults_empty() {
        let json = r#"{
            "data": {
                "type": "review",
                "attributes": {
                    "coverage_score": null,
                    "verdict": null
                }
            }
        }"#;
        let resp: ApiResponse<Review> = serde_json::from_str(json).unwrap();
        let attrs = &resp.data.attributes;
        assert!(attrs.coverage_map.is_empty());
        assert!(attrs.gaps.is_empty());
        assert!(attrs.suggestions.is_empty());
        assert!(attrs.risks.is_empty());
    }

    #[test]
    fn proposal_deserialize() {
        let json = r#"{
            "data": {
                "id": 10,
                "type": "proposal",
                "attributes": {
                    "spec_item_id": 5,
                    "original_description": "User can log in with email.",
                    "proposed_description": "User can log in with email and password. The system validates credentials and returns a JWT.",
                    "rationale": "More detail improves clarity",
                    "status": "pending",
                    "created_at": "2026-03-01T00:00:00Z"
                }
            }
        }"#;
        let resp: ApiResponse<Proposal> = serde_json::from_str(json).unwrap();
        let attrs = &resp.data.attributes;
        assert_eq!(attrs.spec_item_id, 5);
        assert_eq!(attrs.status, "pending");
        assert!(
            attrs
                .proposed_description
                .as_deref()
                .unwrap()
                .contains("JWT")
        );
        assert!(attrs.rationale.is_some());
    }

    #[test]
    fn tech_proposals_deserialize() {
        let json = r#"{
            "data": {
                "functional_item_id": 1,
                "functional_item_permalink": "F-1",
                "proposals": [
                    {
                        "title": "Database schema for users",
                        "description": "Create users table",
                        "type_of": "technical",
                        "rationale": "Required for auth"
                    },
                    {
                        "title": "JWT middleware",
                        "description": null,
                        "type_of": "technical",
                        "rationale": null
                    }
                ]
            }
        }"#;
        let resp: ApiResponse<TechProposals> = serde_json::from_str(json).unwrap();
        let tp = &resp.data;
        assert_eq!(tp.functional_item_id, 1);
        assert_eq!(tp.functional_item_permalink, "F-1");
        assert_eq!(tp.proposals.len(), 2);
        assert_eq!(tp.proposals[0].title, "Database schema for users");
        assert!(tp.proposals[0].rationale.is_some());
        assert!(tp.proposals[1].rationale.is_none());
    }

    #[test]
    fn tech_proposals_empty_proposals_defaults() {
        let json = r#"{
            "data": {
                "functional_item_id": 99,
                "functional_item_permalink": "F-99"
            }
        }"#;
        let resp: ApiResponse<TechProposals> = serde_json::from_str(json).unwrap();
        assert!(resp.data.proposals.is_empty());
    }

    #[test]
    fn usage_filter_query_pairs() {
        let f = UsageFilter {
            month: Some("2026-03".into()),
            page: Some(2),
            per: Some(20),
        };
        let pairs = f.to_query_pairs();
        assert!(pairs.iter().any(|(k, v)| k == "month" && v == "2026-03"));
        assert!(pairs.iter().any(|(k, v)| k == "page" && v == "2"));
        assert!(pairs.iter().any(|(k, v)| k == "per" && v == "20"));
    }

    #[test]
    fn item_filter_to_query_pairs_full() {
        let filter = ItemFilter {
            type_of: Some(ItemType::Functional),
            status: Some(ImplementationStatus::InProgress),
            tag: Some("auth".into()),
            q: Some("login".into()),
            has_review: Some(true),
            review_verdict: Some("approved".into()),
            parent_id: Some(42),
            sort: Some("score".into()),
            page: Some(2),
            per: Some(10),
        };
        let pairs = filter.to_query_pairs();
        assert!(
            pairs
                .iter()
                .any(|(k, v)| k == "type_of" && v == "functional")
        );
        assert!(
            pairs
                .iter()
                .any(|(k, v)| k == "implementation_status" && v == "in_progress")
        );
        assert!(pairs.iter().any(|(k, v)| k == "tag" && v == "auth"));
        assert!(pairs.iter().any(|(k, v)| k == "q" && v == "login"));
        assert!(pairs.iter().any(|(k, v)| k == "has_review" && v == "true"));
        assert!(
            pairs
                .iter()
                .any(|(k, v)| k == "review_verdict" && v == "approved")
        );
        assert!(pairs.iter().any(|(k, v)| k == "parent_id" && v == "42"));
        assert!(pairs.iter().any(|(k, v)| k == "sort" && v == "score"));
        assert!(pairs.iter().any(|(k, v)| k == "page" && v == "2"));
        assert!(pairs.iter().any(|(k, v)| k == "per" && v == "10"));
    }
}

// ---------------------------------------------------------------------------
// Dependency Edge (Graph)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub id: u64,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub attributes: DependencyEdgeAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdgeAttributes {
    pub source_id: u64,
    pub target_id: u64,
    pub edge_type: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateEdgeParams {
    pub source_id: u64,
    pub target_id: u64,
    pub edge_type: String,
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

/// Data returned by the export API.
#[derive(Debug)]
pub enum ExportData {
    /// Text-based format (markdown, json, csv, html).
    Text(String),
    /// Binary format (pdf, docx).
    Binary(Vec<u8>),
}
