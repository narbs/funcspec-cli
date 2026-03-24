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
        Ok(resp.text().await.map_err(Error::from)?)
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
}
