use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// -- API envelope types --

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiListResponse<T> {
    pub data: Vec<T>,
    pub meta: Option<PaginationMeta>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub page: u32,
    pub per: u32,
    pub total: u32,
}

// -- Project --

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: u64,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub attributes: ProjectAttributes,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectAttributes {
    pub name: String,
    pub description: Option<String>,
    pub slug: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// -- Spec Item --

#[derive(Debug, Serialize, Deserialize)]
pub struct SpecItem {
    pub id: u64,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub attributes: SpecItemAttributes,
}

#[derive(Debug, Serialize, Deserialize)]
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

// -- Review --

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewSummary {
    pub id: u64,
    pub coverage_score: Option<f64>,
    pub verdict: Option<String>,
    pub updated_at: DateTime<Utc>,
}

// -- Params for creating/updating items --

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

// -- Query filters --

#[derive(Debug, Default)]
pub struct ItemFilter {
    pub type_of: Option<ItemType>,
    pub status: Option<ImplementationStatus>,
    pub tag: Option<String>,
    pub q: Option<String>,
    pub has_review: Option<bool>,
    pub review_verdict: Option<String>,
    pub parent_id: Option<u64>,
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
        if let Some(p) = self.page {
            pairs.push(("page".into(), p.to_string()));
        }
        if let Some(p) = self.per {
            pairs.push(("per".into(), p.to_string()));
        }
        pairs
    }
}
