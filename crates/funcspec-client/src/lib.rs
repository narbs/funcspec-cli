pub mod client;
pub mod error;
pub mod models;
pub mod pagination;

#[cfg(feature = "blocking")]
pub mod blocking;

pub use client::FuncspecClient;
pub use error::Error;
pub use models::{
    AgentInstructions, AuditResult, CreateItemParams, CreateProjectParams, CreateReviewParams,
    CreateSnapshotParams, ExportData, ImplementationStatus, ItemFilter, ItemType, Job, JobStatus,
    PaginationMeta, Project, ProjectAttributes, ProjectStats, Proposal, ProposalAttributes, Review,
    ReviewAttributes, ReviewSummary, Snapshot, SnapshotAttributes, SnapshotDiff,
    SnapshotDiffEdges, SnapshotDiffItems, SnapshotDiffModified, SnapshotDiffSummary,
    SpecItem, SpecItemAttributes, StatsCoverage, StatsRecentActivity, StatsReviews, StatsSpecItems,
    TechProposal, TechProposals, TokenUsage, UpdateItemParams, UpdateProjectParams, UsageFilter,
    UsageLog, UsageStats, UserInfo,
};
pub use pagination::{PagedResponse, collect_all_pages, stream_all_pages};
