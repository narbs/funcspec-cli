use crate::error::Error;
use crate::models::*;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;

/// FuncSpec API client.
pub struct FuncspecClient {
    http: Client,
    base_url: String,
}

impl FuncspecClient {
    /// Create a new client for the given host and API key.
    pub fn new(base_url: &str, api_key: &str) -> Result<Self, Error> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Api-Key",
            HeaderValue::from_str(api_key).map_err(|e| Error::Other(e.to_string()))?,
        );

        let http = Client::builder()
            .default_headers(headers)
            .user_agent(format!("funcspec-cli/{}", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let base_url = base_url.trim_end_matches('/').to_string();

        Ok(Self { http, base_url })
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/api/v1{}", self.base_url, path)
    }

    /// Validate the API key by hitting the ping or projects endpoint.
    pub async fn validate_auth(&self) -> Result<(), Error> {
        let resp = self.http.get(self.api_url("/projects")).send().await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(Error::from_response(resp).await)
        }
    }

    // -- Projects --

    pub async fn list_projects(&self) -> Result<Vec<Project>, Error> {
        let resp = self.http.get(self.api_url("/projects")).send().await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiListResponse<Project> = resp.json().await?;
        Ok(body.data)
    }

    pub async fn get_project(&self, slug_or_id: &str) -> Result<Project, Error> {
        let resp = self
            .http
            .get(self.api_url(&format!("/projects/{slug_or_id}")))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<Project> = resp.json().await?;
        Ok(body.data)
    }

    // -- Items --

    pub async fn list_items(
        &self,
        project_id: u64,
        filter: &ItemFilter,
    ) -> Result<(Vec<SpecItem>, Option<PaginationMeta>), Error> {
        let url = self.api_url(&format!("/projects/{project_id}/spec/items"));
        let resp = self
            .http
            .get(&url)
            .query(&filter.to_query_pairs())
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiListResponse<SpecItem> = resp.json().await?;
        Ok((body.data, body.meta))
    }

    pub async fn get_item(&self, project_id: u64, id_or_permalink: &str) -> Result<SpecItem, Error> {
        let resp = self
            .http
            .get(self.api_url(&format!(
                "/projects/{project_id}/spec/items/{id_or_permalink}"
            )))
            .send()
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
        let resp = self
            .http
            .post(self.api_url(&format!("/projects/{project_id}/spec/items")))
            .json(params)
            .send()
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
        let resp = self
            .http
            .patch(self.api_url(&format!(
                "/projects/{project_id}/spec/items/{item_id}"
            )))
            .json(params)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        let body: ApiResponse<SpecItem> = resp.json().await?;
        Ok(body.data)
    }

    pub async fn delete_item(&self, project_id: u64, item_id: u64) -> Result<(), Error> {
        let resp = self
            .http
            .delete(self.api_url(&format!(
                "/projects/{project_id}/spec/items/{item_id}"
            )))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        Ok(())
    }
}
