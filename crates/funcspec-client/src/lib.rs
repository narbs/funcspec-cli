pub mod client;
pub mod error;
pub mod models;
pub mod pagination;

#[cfg(feature = "blocking")]
pub mod blocking;

pub use client::FuncspecClient;
pub use error::Error;
pub use models::{
    AuditResult, CreateItemParams, CreateProjectParams, CreateReviewParams, ImplementationStatus,
    ItemFilter, ItemType, Job, JobStatus, PaginationMeta, Project, ProjectAttributes, Review,
    ReviewStatus, ReviewSummary, SpecItem, SpecItemAttributes, Snapshot, UpdateItemParams,
    UpdateProjectParams, UsageLog, UserInfo,
};
pub use pagination::{collect_all_pages, stream_all_pages, PagedResponse};
