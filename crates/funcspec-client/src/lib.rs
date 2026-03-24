pub mod client;
pub mod error;
pub mod models;
pub mod pagination;

#[cfg(feature = "blocking")]
pub mod blocking;

pub use client::FuncspecClient;
pub use error::Error;
pub use models::{
    AuditResult, CreateItemParams, CreateProjectParams, CreateReviewParams, ExportData,
    ImplementationStatus, ItemFilter, ItemType, Job, JobStatus, PaginationMeta, Project,
    ProjectAttributes, ProjectStats, Proposal, ProposalAttributes, RecentActivity, Review,
    ReviewCoverage, ReviewStatus, ReviewSummary, SpecItem, SpecItemAttributes, Snapshot,
    TechProposal, TechProposals, TokenUsage, UpdateItemParams, UpdateProjectParams, UsageFilter,
    UsageLog, UsageStats, UserInfo, VerdictDistribution,
};
pub use pagination::{collect_all_pages, stream_all_pages, PagedResponse};
