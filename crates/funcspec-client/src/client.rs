use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use tracing::{debug, warn};

use crate::error::Error;
use crate::models::*;
use crate::pagination::PagedResponse;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_RETRIES: u32 = 3;

/// FuncSpec API client.
#[derive(Clone)]
pub struct FuncspecClient {
    http: Client,
    base_url: String,
    max_retries: u32,
}

impl FuncspecClient {
    /// Create a new client for the given host and API key.
    pub fn new(base_url: &str, api_key: &str) -> Result<Self, Error> {
        Self::with_timeout(base_url, api_key, Duration::from_secs(DEFAULT_TIMEOUT_SECS))
    }

    /// Create a client with a custom request timeout.
    pub fn with_timeout(base_url: &str, api_key: &str, timeout: Duration) -> Result<Self, Error> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Api-Key",
            HeaderValue::from_str(api_key).map_err(|e| Error::Other(e.to_string()))?,
        );

        let http = Client::builder()
            .default_headers(headers)
            .user_agent(format!("funcspec-cli/{}", env!("CARGO_PKG_VERSION")))
            .timeout(timeout)
            .build()?;

        let base_url = base_url.trim_end_matches('/').to_string();

        Ok(Self {
            http,
            base_url,
            max_retries: MAX_RETRIES,
        })
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/api/v1{}", self.base_url, path)
    }

    /// Send a request with automatic retry on rate-limit (429) and transient errors.
    ///
    /// Uses exponential backoff: 1 s, 2 s, 4 s (capped at 60 s).
    async fn request_with_retry<F, Fut>(&self, make_request: F) -> Result<reqwest::Response, Error>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
    {
        let mut attempt = 0u32;
        loop {
            match make_request().await {
                Ok(resp) => {
                    if resp.status().as_u16() == 429 && attempt < self.max_retries {
                        let retry_after = resp
                            .headers()
                            .get("retry-after")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or_else(|| {
                                // Exponential backoff: 1, 2, 4, ...
                                1u64 << attempt.min(6)
                            });
                        let delay = Duration::from_secs(retry_after.min(60));
                        warn!(attempt, retry_after_secs = retry_after, "Rate limited, backing off");
                        tokio::time::sleep(delay).await;
                        attempt += 1;
                        continue;
                    }
                    return Ok(resp);
                }
                Err(e) if attempt < self.max_retries => {
                    // Retry on network errors
                    if e.is_connect() || e.is_timeout() {
                        let delay = Duration::from_secs(1u64 << attempt.min(6));
                        warn!(attempt, error = %e, "Network error, retrying");
                        tokio::time::sleep(delay).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(Error::from(e));
                }
                Err(e) => return Err(Error::from(e)),
            }
        }
    }

    // -- Auth --

    /// Validate the API key and return user/org info.
    pub async fn validate_auth(&self) -> Result<UserInfo, Error> {
        let url = self.api_url("/auth/validate");
        debug!(%url, "validate_auth");
        let resp = self
            .request_with_retry(|| self.http.get(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<UserInfo> = resp.json().await?;
        Ok(body.data)
    }

    // -- Projects --

    pub async fn list_projects(&self) -> Result<Vec<Project>, Error> {
        let url = self.api_url("/projects");
        debug!(%url, "list_projects");
        let resp = self
            .request_with_retry(|| self.http.get(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiListResponse<Project> = resp.json().await?;
        Ok(body.data)
    }

    pub async fn list_projects_paged(
        &self,
        page: u32,
        per_page: u32,
    ) -> Result<PagedResponse<Project>, Error> {
        let url = self.api_url("/projects");
        let resp = self
            .request_with_retry(|| {
                self.http
                    .get(&url)
                    .query(&[("page", page), ("per", per_page)])
                    .send()
            })
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiListResponse<Project> = resp.json().await?;
        Ok((body.data, body.meta).into())
    }

    pub async fn get_project(&self, slug_or_id: &str) -> Result<Project, Error> {
        let url = self.api_url(&format!("/projects/{slug_or_id}"));
        debug!(%url, "get_project");
        let resp = self
            .request_with_retry(|| self.http.get(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<Project> = resp.json().await?;
        Ok(body.data)
    }

    pub async fn create_project(&self, params: &CreateProjectParams) -> Result<Project, Error> {
        let url = self.api_url("/projects");
        let resp = self
            .request_with_retry(|| self.http.post(&url).json(params).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<Project> = resp.json().await?;
        Ok(body.data)
    }

    pub async fn delete_project(&self, slug_or_id: &str) -> Result<(), Error> {
        let url = self.api_url(&format!("/projects/{slug_or_id}"));
        let resp = self
            .request_with_retry(|| self.http.delete(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        Ok(())
    }

    // -- Items --

    pub async fn list_items(
        &self,
        project_id: u64,
        filter: &ItemFilter,
    ) -> Result<(Vec<SpecItem>, Option<PaginationMeta>), Error> {
        let url = self.api_url(&format!("/projects/{project_id}/spec/items"));
        debug!(%url, "list_items");
        let resp = self
            .request_with_retry(|| {
                self.http
                    .get(&url)
                    .query(&filter.to_query_pairs())
                    .send()
            })
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiListResponse<SpecItem> = resp.json().await?;
        Ok((body.data, body.meta))
    }

    pub async fn list_items_paged(
        &self,
        project_id: u64,
        filter: &ItemFilter,
        page: u32,
        per_page: u32,
    ) -> Result<PagedResponse<SpecItem>, Error> {
        let url = self.api_url(&format!("/projects/{project_id}/spec/items"));
        let mut pairs = filter.to_query_pairs();
        pairs.push(("page".into(), page.to_string()));
        pairs.push(("per".into(), per_page.to_string()));
        let resp = self
            .request_with_retry(|| self.http.get(&url).query(&pairs).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiListResponse<SpecItem> = resp.json().await?;
        Ok((body.data, body.meta).into())
    }

    /// Search spec items by full-text query, with optional type/tag filters.
    ///
    /// Calls GET /projects/{project_id}/spec/items?q=<query>[&type_of=...][&tag=...]
    pub async fn search_items(
        &self,
        project_id: u64,
        query: &str,
        filter: &ItemFilter,
    ) -> Result<PagedResponse<SpecItem>, Error> {
        let url = self.api_url(&format!("/projects/{project_id}/spec/items"));
        debug!(%url, %query, "search_items");
        // Build query pairs from filter, then force-set q to the search query
        let mut pairs = filter.to_query_pairs();
        pairs.retain(|(k, _)| k != "q");
        pairs.push(("q".into(), query.to_string()));
        let resp = self
            .request_with_retry(|| self.http.get(&url).query(&pairs).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiListResponse<SpecItem> = resp.json().await?;
        Ok((body.data, body.meta).into())
    }

    pub async fn get_item(
        &self,
        project_id: u64,
        id_or_permalink: &str,
    ) -> Result<SpecItem, Error> {
        let url = self.api_url(&format!(
            "/projects/{project_id}/spec/items/{id_or_permalink}"
        ));
        debug!(%url, "get_item");
        let resp = self
            .request_with_retry(|| self.http.get(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<SpecItem> = resp.json().await?;
        Ok(body.data)
    }

    pub async fn create_item(
        &self,
        project_id: u64,
        params: &CreateItemParams,
    ) -> Result<SpecItem, Error> {
        let url = self.api_url(&format!("/projects/{project_id}/spec/items"));
        let resp = self
            .request_with_retry(|| self.http.post(&url).json(params).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<SpecItem> = resp.json().await?;
        Ok(body.data)
    }

    pub async fn update_item(
        &self,
        project_id: u64,
        item_id: u64,
        params: &UpdateItemParams,
    ) -> Result<SpecItem, Error> {
        let url = self.api_url(&format!(
            "/projects/{project_id}/spec/items/{item_id}"
        ));
        let resp = self
            .request_with_retry(|| self.http.patch(&url).json(params).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<SpecItem> = resp.json().await?;
        Ok(body.data)
    }

    pub async fn delete_item(&self, project_id: u64, item_id: u64) -> Result<(), Error> {
        let url = self.api_url(&format!(
            "/projects/{project_id}/spec/items/{item_id}"
        ));
        let resp = self
            .request_with_retry(|| self.http.delete(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        Ok(())
    }

    // -- Reviews --

    pub async fn list_reviews(&self, spec_item_id: u64) -> Result<Vec<Review>, Error> {
        let url = self.api_url(&format!("/spec/items/{spec_item_id}/reviews"));
        let resp = self
            .request_with_retry(|| self.http.get(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiListResponse<Review> = resp.json().await?;
        Ok(body.data)
    }

    pub async fn create_review(
        &self,
        spec_item_id: u64,
        params: &CreateReviewParams,
    ) -> Result<Review, Error> {
        let url = self.api_url(&format!("/spec/items/{spec_item_id}/reviews"));
        let resp = self
            .request_with_retry(|| self.http.post(&url).json(params).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<Review> = resp.json().await?;
        Ok(body.data)
    }

    // -- Jobs --

    pub async fn list_jobs(&self) -> Result<Vec<Job>, Error> {
        let url = self.api_url("/jobs");
        let resp = self
            .request_with_retry(|| self.http.get(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiListResponse<Job> = resp.json().await?;
        Ok(body.data)
    }

    pub async fn get_job(&self, job_id: u64) -> Result<Job, Error> {
        let url = self.api_url(&format!("/jobs/{job_id}"));
        let resp = self
            .request_with_retry(|| self.http.get(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<Job> = resp.json().await?;
        Ok(body.data)
    }

    // -- AI Operations --

    /// Trigger AI review of a single spec item.
    ///
    /// Calls `POST /projects/:project_id/work_package/:item_id/review`.
    pub async fn review_item(&self, project_id: u64, item_id: u64) -> Result<Review, Error> {
        let url = self.api_url(&format!(
            "/projects/{project_id}/work_package/{item_id}/review"
        ));
        debug!(%url, "review_item");
        let resp = self
            .request_with_retry(|| self.http.post(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<Review> = resp.json().await?;
        Ok(body.data)
    }

    /// Trigger batch AI review of all items in a project.
    ///
    /// Calls `POST /projects/:project_id/work_package/review_all` and returns an
    /// async [`Job`] that can be polled via [`poll_job_until_done`].
    pub async fn review_all(&self, project_id: u64) -> Result<Job, Error> {
        let url = self.api_url(&format!("/projects/{project_id}/work_package/review_all"));
        debug!(%url, "review_all");
        let resp = self
            .request_with_retry(|| self.http.post(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<Job> = resp.json().await?;
        Ok(body.data)
    }

    /// Propose an AI-generated improvement for a spec item.
    ///
    /// Calls `POST /projects/:project_id/work_package/:item_id/propose`.
    pub async fn propose_improvement(
        &self,
        project_id: u64,
        item_id: u64,
    ) -> Result<Proposal, Error> {
        let url = self.api_url(&format!(
            "/projects/{project_id}/work_package/{item_id}/propose"
        ));
        debug!(%url, "propose_improvement");
        let resp = self
            .request_with_retry(|| self.http.post(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<Proposal> = resp.json().await?;
        Ok(body.data)
    }

    /// Accept a pending improvement proposal for a spec item.
    ///
    /// Calls `POST /projects/:project_id/work_package/:item_id/accept`.
    pub async fn accept_proposal(&self, project_id: u64, item_id: u64) -> Result<(), Error> {
        let url = self.api_url(&format!(
            "/projects/{project_id}/work_package/{item_id}/accept"
        ));
        debug!(%url, "accept_proposal");
        let resp = self
            .request_with_retry(|| self.http.post(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        Ok(())
    }

    /// Reject a pending improvement proposal for a spec item.
    ///
    /// Calls `POST /projects/:project_id/work_package/:item_id/reject`.
    pub async fn reject_proposal(&self, project_id: u64, item_id: u64) -> Result<(), Error> {
        let url = self.api_url(&format!(
            "/projects/{project_id}/work_package/{item_id}/reject"
        ));
        debug!(%url, "reject_proposal");
        let resp = self
            .request_with_retry(|| self.http.post(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        Ok(())
    }

    /// Generate technical spec proposals from a functional spec item.
    ///
    /// Calls `POST /projects/:project_id/work_package/:item_id/generate_tech`.
    pub async fn generate_tech(
        &self,
        project_id: u64,
        item_id: u64,
    ) -> Result<TechProposals, Error> {
        let url = self.api_url(&format!(
            "/projects/{project_id}/work_package/{item_id}/generate_tech"
        ));
        debug!(%url, "generate_tech");
        let resp = self
            .request_with_retry(|| self.http.post(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<TechProposals> = resp.json().await?;
        Ok(body.data)
    }

    /// Run a code audit for a spec item.
    ///
    /// Calls `POST /projects/:project_id/work_package/:item_id/audit`.
    pub async fn audit_item(&self, project_id: u64, item_id: u64) -> Result<AuditResult, Error> {
        let url = self.api_url(&format!(
            "/projects/{project_id}/work_package/{item_id}/audit"
        ));
        debug!(%url, "audit_item");
        let resp = self
            .request_with_retry(|| self.http.post(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<AuditResult> = resp.json().await?;
        Ok(body.data)
    }

    /// Fetch a job by ID within a project context.
    ///
    /// Calls `GET /projects/:project_id/jobs/:job_id`.
    pub async fn get_project_job(&self, project_id: u64, job_id: u64) -> Result<Job, Error> {
        let url = self.api_url(&format!("/projects/{project_id}/jobs/{job_id}"));
        debug!(%url, "get_project_job");
        let resp = self
            .request_with_retry(|| self.http.get(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<Job> = resp.json().await?;
        Ok(body.data)
    }

    /// Poll a job until it reaches a terminal state (Completed or Failed).
    ///
    /// Returns the final [`Job`] on completion, or [`Error::Timeout`] if `timeout`
    /// elapses before the job finishes. Polls every 500 ms.
    pub async fn poll_job_until_done(
        &self,
        job_id: u64,
        timeout: Duration,
    ) -> Result<Job, Error> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(500);
        loop {
            let job = self.get_job(job_id).await?;
            match job.attributes.status {
                JobStatus::Completed | JobStatus::Failed => return Ok(job),
                _ => {}
            }
            if start.elapsed() >= timeout {
                return Err(Error::Timeout {
                    secs: timeout.as_secs(),
                });
            }
            tokio::time::sleep(poll_interval).await;
        }
    }

    // -- Usage --

    pub async fn list_usage(&self) -> Result<Vec<UsageLog>, Error> {
        let url = self.api_url("/usage");
        let resp = self
            .request_with_retry(|| self.http.get(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiListResponse<UsageLog> = resp.json().await?;
        Ok(body.data)
    }

    // -- Stats --

    /// Fetch aggregated dashboard stats for a project.
    pub async fn get_project_stats(&self, project_id: u64) -> Result<ProjectStats, Error> {
        let url = self.api_url(&format!("/projects/{project_id}/stats"));
        debug!(%url, "get_project_stats");
        let resp = self
            .request_with_retry(|| self.http.get(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<ProjectStats> = resp.json().await?;
        Ok(body.data)
    }

    /// Fetch LLM usage logs for a project, with optional month filter.
    pub async fn get_usage_logs(
        &self,
        project_id: u64,
        filter: &UsageFilter,
    ) -> Result<PagedResponse<UsageLog>, Error> {
        let url = self.api_url(&format!("/projects/{project_id}/usage/logs"));
        debug!(%url, "get_usage_logs");
        let pairs = filter.to_query_pairs();
        let resp = self
            .request_with_retry(|| self.http.get(&url).query(&pairs).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiListResponse<UsageLog> = resp.json().await?;
        Ok((body.data, body.meta).into())
    }

    /// Fetch LLM usage summary for a project. Pass `month` as "YYYY-MM" to filter.
    pub async fn get_usage_stats(
        &self,
        project_id: u64,
        month: Option<&str>,
    ) -> Result<UsageStats, Error> {
        let url = self.api_url(&format!("/projects/{project_id}/usage"));
        debug!(%url, "get_usage_stats");
        let mut pairs: Vec<(String, String)> = Vec::new();
        if let Some(m) = month {
            pairs.push(("month".into(), m.to_string()));
        }
        let resp = self
            .request_with_retry(|| self.http.get(&url).query(&pairs).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<UsageStats> = resp.json().await?;
        Ok(body.data)
    }

    // -- Export --

    /// Export a project spec in the specified format.
    ///
    /// Text formats (`markdown`, `json`, `csv`, `html`) return [`ExportData::Text`].
    /// Binary formats (`pdf`, `docx`) return [`ExportData::Binary`].
    ///
    /// Optional `item_type` (`"functional"` / `"technical"`) and `tag` filters are
    /// forwarded as query parameters to the API.
    pub async fn export_project(
        &self,
        project_id: u64,
        format: &str,
        item_type: Option<&str>,
        tag: Option<&str>,
    ) -> Result<ExportData, Error> {
        let url = self.api_url(&format!("/projects/{project_id}/spec/export"));
        debug!(%url, %format, "export_project");
        let mut pairs: Vec<(String, String)> = vec![("format".into(), format.to_string())];
        if let Some(t) = item_type {
            pairs.push(("type_of".into(), t.to_string()));
        }
        if let Some(t) = tag {
            pairs.push(("tag".into(), t.to_string()));
        }
        let resp = self
            .request_with_retry(|| self.http.get(&url).query(&pairs).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let is_binary = matches!(format, "pdf" | "docx");
        if is_binary {
            let bytes = resp.bytes().await.map_err(Error::from)?;
            Ok(ExportData::Binary(bytes.to_vec()))
        } else {
            let text = resp.text().await.map_err(Error::from)?;
            Ok(ExportData::Text(text))
        }
    }

    /// Fetch the viewable HTML spec for a project.
    ///
    /// Calls `GET /projects/:id/spec/view` and returns the response body as a
    /// UTF-8 string.
    pub async fn get_viewable_html(&self, project_id: u64) -> Result<String, Error> {
        let url = self.api_url(&format!("/projects/{project_id}/spec/view"));
        debug!(%url, "get_viewable_html");
        let resp = self
            .request_with_retry(|| self.http.get(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        resp.text().await.map_err(Error::from)
    }

    // -- Snapshots --

    /// List all snapshots for a project.
    ///
    /// Calls `GET /projects/:project_id/snapshots`.
    pub async fn list_snapshots(&self, project_id: u64) -> Result<Vec<Snapshot>, Error> {
        let url = self.api_url(&format!("/projects/{project_id}/snapshots"));
        debug!(%url, "list_snapshots");
        let resp = self
            .request_with_retry(|| self.http.get(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiListResponse<Snapshot> = resp.json().await?;
        Ok(body.data)
    }

    /// Create a new snapshot for a project.
    ///
    /// Calls `POST /projects/:project_id/snapshots`.
    pub async fn create_snapshot(
        &self,
        project_id: u64,
        params: &CreateSnapshotParams,
    ) -> Result<Snapshot, Error> {
        let url = self.api_url(&format!("/projects/{project_id}/snapshots"));
        debug!(%url, "create_snapshot");
        let resp = self
            .request_with_retry(|| self.http.post(&url).json(params).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<Snapshot> = resp.json().await?;
        Ok(body.data)
    }

    /// Get a single snapshot by ID.
    ///
    /// Calls `GET /projects/:project_id/snapshots/:snapshot_id`.
    pub async fn get_snapshot(
        &self,
        project_id: u64,
        snapshot_id: u64,
    ) -> Result<Snapshot, Error> {
        let url = self.api_url(&format!("/projects/{project_id}/snapshots/{snapshot_id}"));
        debug!(%url, "get_snapshot");
        let resp = self
            .request_with_retry(|| self.http.get(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<Snapshot> = resp.json().await?;
        Ok(body.data)
    }

    /// Restore project to the state captured in a snapshot.
    ///
    /// Calls `POST /projects/:project_id/snapshots/:snapshot_id/restore`.
    pub async fn restore_snapshot(
        &self,
        project_id: u64,
        snapshot_id: u64,
    ) -> Result<(), Error> {
        let url =
            self.api_url(&format!("/projects/{project_id}/snapshots/{snapshot_id}/restore"));
        debug!(%url, "restore_snapshot");
        let resp = self
            .request_with_retry(|| self.http.post(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        Ok(())
    }

    /// Delete a snapshot.
    ///
    /// Calls `DELETE /projects/:project_id/snapshots/:snapshot_id`.
    pub async fn delete_snapshot(
        &self,
        project_id: u64,
        snapshot_id: u64,
    ) -> Result<(), Error> {
        let url = self.api_url(&format!("/projects/{project_id}/snapshots/{snapshot_id}"));
        debug!(%url, "delete_snapshot");
        let resp = self
            .request_with_retry(|| self.http.delete(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        Ok(())
    }

    /// Get the diff between a snapshot and the current project state.
    ///
    /// Calls `GET /projects/:project_id/snapshots/:snapshot_id/diff`.
    pub async fn diff_snapshot(
        &self,
        project_id: u64,
        snapshot_id: u64,
    ) -> Result<SnapshotDiff, Error> {
        let url =
            self.api_url(&format!("/projects/{project_id}/snapshots/{snapshot_id}/diff"));
        debug!(%url, "diff_snapshot");
        let resp = self
            .request_with_retry(|| self.http.get(&url).send())
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<SnapshotDiff> = resp.json().await?;
        Ok(body.data)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn make_client(base_url: &str) -> FuncspecClient {
        FuncspecClient::new(base_url, "test-key").unwrap()
    }

    #[tokio::test]
    async fn export_project_text_format() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/projects/42/spec/export"))
            .and(query_param("format", "markdown"))
            .respond_with(ResponseTemplate::new(200).set_body_string("# My Spec\n\nContent here."))
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        let result = client.export_project(42, "markdown", None, None).await.unwrap();
        match result {
            ExportData::Text(text) => assert!(text.contains("# My Spec")),
            ExportData::Binary(_) => panic!("expected text"),
        }
    }

    #[tokio::test]
    async fn export_project_with_filters() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/projects/42/spec/export"))
            .and(query_param("format", "csv"))
            .and(query_param("type_of", "functional"))
            .and(query_param("tag", "v1"))
            .respond_with(ResponseTemplate::new(200).set_body_string("id,title\n1,Login"))
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        let result = client
            .export_project(42, "csv", Some("functional"), Some("v1"))
            .await
            .unwrap();
        match result {
            ExportData::Text(text) => assert!(text.contains("id,title")),
            ExportData::Binary(_) => panic!("expected text"),
        }
    }

    #[tokio::test]
    async fn export_project_binary_format() {
        let server = MockServer::start().await;
        let pdf_bytes = b"%PDF-1.4 fake pdf content".to_vec();
        Mock::given(method("GET"))
            .and(path("/api/v1/projects/7/spec/export"))
            .and(query_param("format", "pdf"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(pdf_bytes.clone()))
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        let result = client.export_project(7, "pdf", None, None).await.unwrap();
        match result {
            ExportData::Binary(bytes) => assert_eq!(bytes, pdf_bytes),
            ExportData::Text(_) => panic!("expected binary"),
        }
    }

    #[tokio::test]
    async fn export_project_docx_binary() {
        let server = MockServer::start().await;
        let docx_bytes = b"PK\x03\x04fake docx".to_vec();
        Mock::given(method("GET"))
            .and(path("/api/v1/projects/5/spec/export"))
            .and(query_param("format", "docx"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(docx_bytes.clone()))
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        let result = client.export_project(5, "docx", None, None).await.unwrap();
        match result {
            ExportData::Binary(bytes) => assert_eq!(bytes, docx_bytes),
            ExportData::Text(_) => panic!("expected binary"),
        }
    }

    #[tokio::test]
    async fn export_project_error_response() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/projects/99/spec/export"))
            .respond_with(
                ResponseTemplate::new(404)
                    .set_body_json(serde_json::json!({"error": "Project not found"})),
            )
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        let result = client.export_project(99, "markdown", None, None).await;
        assert!(matches!(result, Err(Error::NotFound(_))));
    }

    #[tokio::test]
    async fn get_viewable_html_returns_string() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/projects/3/spec/view"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("<html><body>Spec</body></html>"),
            )
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        let html = client.get_viewable_html(3).await.unwrap();
        assert!(html.contains("<html>"));
    }

    #[tokio::test]
    async fn get_viewable_html_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/projects/0/spec/view"))
            .respond_with(
                ResponseTemplate::new(403)
                    .set_body_json(serde_json::json!({"error": "Forbidden"})),
            )
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        let result = client.get_viewable_html(0).await;
        assert!(matches!(result, Err(Error::Forbidden(_))));
    }

    // -- AI Operations --

    #[tokio::test]
    async fn review_item_returns_review() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/projects/1/work_package/5/review"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "id": 42,
                    "type": "review",
                    "attributes": {
                        "spec_item_id": 5,
                        "reviewer": "ai",
                        "status": "approved",
                        "comment": "Well defined",
                        "coverage_score": 90.0,
                        "verdict": "pass",
                        "coverage_map": ["Auth flow", "Error cases"],
                        "gaps": [],
                        "suggestions": ["Consider adding retry logic"],
                        "created_at": "2026-01-01T00:00:00Z",
                        "updated_at": "2026-01-01T00:00:00Z"
                    }
                }
            })))
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        let review = client.review_item(1, 5).await.unwrap();
        assert_eq!(review.attributes.coverage_score, Some(90.0));
        assert_eq!(review.attributes.verdict.as_deref(), Some("pass"));
        assert_eq!(review.attributes.coverage_map.len(), 2);
        assert!(review.attributes.gaps.is_empty());
        assert_eq!(review.attributes.suggestions.len(), 1);
    }

    #[tokio::test]
    async fn review_item_not_found() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/projects/1/work_package/999/review"))
            .respond_with(
                ResponseTemplate::new(404)
                    .set_body_json(serde_json::json!({"error": "Not found"})),
            )
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        let result = client.review_item(1, 999).await;
        assert!(matches!(result, Err(Error::NotFound(_))));
    }

    #[tokio::test]
    async fn review_all_returns_job() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/projects/2/work_package/review_all"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "id": 77,
                    "type": "job",
                    "attributes": {
                        "job_type": "batch_review",
                        "status": "pending",
                        "progress": null,
                        "result": null,
                        "created_at": "2026-01-01T00:00:00Z",
                        "updated_at": "2026-01-01T00:00:00Z"
                    }
                }
            })))
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        let job = client.review_all(2).await.unwrap();
        assert_eq!(job.id, 77);
        assert_eq!(job.attributes.job_type, "batch_review");
        assert_eq!(job.attributes.status, JobStatus::Pending);
    }

    #[tokio::test]
    async fn propose_improvement_returns_proposal() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/projects/3/work_package/10/propose"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "id": 1,
                    "type": "proposal",
                    "attributes": {
                        "spec_item_id": 10,
                        "original_description": "User logs in.",
                        "proposed_description": "User logs in with email and password.",
                        "rationale": "More specific",
                        "status": "pending",
                        "created_at": "2026-01-01T00:00:00Z"
                    }
                }
            })))
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        let proposal = client.propose_improvement(3, 10).await.unwrap();
        assert_eq!(proposal.attributes.spec_item_id, 10);
        assert_eq!(proposal.attributes.status, "pending");
        assert!(proposal
            .attributes
            .proposed_description
            .unwrap()
            .contains("email"));
    }

    #[tokio::test]
    async fn accept_proposal_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/projects/4/work_package/20/accept"))
            .respond_with(ResponseTemplate::new(200).set_body_string(""))
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        assert!(client.accept_proposal(4, 20).await.is_ok());
    }

    #[tokio::test]
    async fn reject_proposal_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/projects/4/work_package/20/reject"))
            .respond_with(ResponseTemplate::new(200).set_body_string(""))
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        assert!(client.reject_proposal(4, 20).await.is_ok());
    }

    #[tokio::test]
    async fn generate_tech_returns_proposals() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/projects/5/work_package/30/generate_tech"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "functional_item_id": 30,
                    "functional_item_permalink": "F-30",
                    "proposals": [
                        {
                            "title": "Users table schema",
                            "description": "Create users table",
                            "type_of": "technical",
                            "rationale": "Required for auth"
                        },
                        {
                            "title": "JWT service",
                            "description": null,
                            "type_of": "technical",
                            "rationale": null
                        }
                    ]
                }
            })))
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        let tp = client.generate_tech(5, 30).await.unwrap();
        assert_eq!(tp.functional_item_id, 30);
        assert_eq!(tp.functional_item_permalink, "F-30");
        assert_eq!(tp.proposals.len(), 2);
        assert_eq!(tp.proposals[0].title, "Users table schema");
    }

    #[tokio::test]
    async fn audit_item_returns_result() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/projects/6/work_package/40/audit"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "id": 55,
                    "type": "audit_result",
                    "attributes": {
                        "spec_item_id": 40,
                        "audit_type": "coverage",
                        "passed": true,
                        "details": "All requirements covered",
                        "created_at": "2026-01-01T00:00:00Z"
                    }
                }
            })))
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        let audit = client.audit_item(6, 40).await.unwrap();
        assert!(audit.attributes.passed);
        assert_eq!(audit.attributes.audit_type, "coverage");
    }

    #[tokio::test]
    async fn get_project_job_returns_job() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/projects/7/jobs/88"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "id": 88,
                    "type": "job",
                    "attributes": {
                        "job_type": "review",
                        "status": "completed",
                        "progress": 100.0,
                        "result": "All items reviewed",
                        "created_at": "2026-01-01T00:00:00Z",
                        "updated_at": "2026-01-01T00:01:00Z"
                    }
                }
            })))
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        let job = client.get_project_job(7, 88).await.unwrap();
        assert_eq!(job.id, 88);
        assert_eq!(job.attributes.status, JobStatus::Completed);
    }

    #[tokio::test]
    async fn poll_job_until_done_completes_immediately() {
        let server = MockServer::start().await;
        // The job is already completed on first poll
        Mock::given(method("GET"))
            .and(path("/api/v1/jobs/99"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "id": 99,
                    "type": "job",
                    "attributes": {
                        "job_type": "review",
                        "status": "completed",
                        "progress": 100.0,
                        "result": "Done",
                        "created_at": "2026-01-01T00:00:00Z",
                        "updated_at": "2026-01-01T00:00:30Z"
                    }
                }
            })))
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        let job = client
            .poll_job_until_done(99, Duration::from_secs(30))
            .await
            .unwrap();
        assert_eq!(job.attributes.status, JobStatus::Completed);
    }

    #[tokio::test]
    async fn poll_job_until_done_returns_failed_job() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/jobs/100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "id": 100,
                    "type": "job",
                    "attributes": {
                        "job_type": "audit",
                        "status": "failed",
                        "progress": null,
                        "result": "Internal error",
                        "created_at": "2026-01-01T00:00:00Z",
                        "updated_at": "2026-01-01T00:00:05Z"
                    }
                }
            })))
            .mount(&server)
            .await;

        let client = make_client(&server.uri()).await;
        let job = client
            .poll_job_until_done(100, Duration::from_secs(30))
            .await
            .unwrap();
        // poll_job_until_done returns the job even when failed
        assert_eq!(job.attributes.status, JobStatus::Failed);
    }
}
