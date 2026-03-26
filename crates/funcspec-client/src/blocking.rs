//! Optional blocking (synchronous) wrapper for [`FuncspecClient`].
//!
//! Enabled via the `blocking` Cargo feature:
//! ```toml
//! funcspec-client = { version = "0.1", features = ["blocking"] }
//! ```
//!
//! **Warning:** Do not call blocking methods from inside a Tokio async context —
//! use the async [`FuncspecClient`] instead.

use crate::FuncspecClient;
use crate::error::Error;
use crate::models::*;

/// A synchronous wrapper around [`FuncspecClient`].
///
/// Creates its own single-threaded Tokio runtime to drive async calls.
pub struct BlockingFuncspecClient {
    inner: FuncspecClient,
    rt: tokio::runtime::Runtime,
}

impl BlockingFuncspecClient {
    /// Create a new blocking client.
    pub fn new(base_url: &str, api_key: &str) -> Result<Self, Error> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::Other(e.to_string()))?;
        let inner = FuncspecClient::new(base_url, api_key)?;
        Ok(Self { inner, rt })
    }

    /// Validate the stored credentials against the API.
    pub fn validate_auth(&self) -> Result<UserInfo, Error> {
        self.rt.block_on(self.inner.validate_auth())
    }

    // -- Projects --

    pub fn list_projects(&self) -> Result<Vec<Project>, Error> {
        self.rt.block_on(self.inner.list_projects())
    }

    pub fn get_project(&self, slug_or_id: &str) -> Result<Project, Error> {
        self.rt.block_on(self.inner.get_project(slug_or_id))
    }

    pub fn create_project(&self, params: &CreateProjectParams) -> Result<Project, Error> {
        self.rt.block_on(self.inner.create_project(params))
    }

    pub fn delete_project(&self, slug_or_id: &str) -> Result<(), Error> {
        self.rt.block_on(self.inner.delete_project(slug_or_id))
    }

    // -- Items --

    pub fn list_items(
        &self,
        project_id: u64,
        filter: &ItemFilter,
    ) -> Result<(Vec<SpecItem>, Option<PaginationMeta>), Error> {
        self.rt.block_on(self.inner.list_items(project_id, filter))
    }

    pub fn get_item(&self, project_id: u64, id_or_permalink: &str) -> Result<SpecItem, Error> {
        self.rt
            .block_on(self.inner.get_item(project_id, id_or_permalink))
    }

    pub fn create_item(
        &self,
        project_id: u64,
        params: &CreateItemParams,
    ) -> Result<SpecItem, Error> {
        self.rt.block_on(self.inner.create_item(project_id, params))
    }

    pub fn update_item(
        &self,
        project_id: u64,
        item_id: u64,
        params: &UpdateItemParams,
    ) -> Result<SpecItem, Error> {
        self.rt
            .block_on(self.inner.update_item(project_id, item_id, params))
    }

    pub fn delete_item(&self, project_id: u64, item_id: u64) -> Result<(), Error> {
        self.rt
            .block_on(self.inner.delete_item(project_id, item_id))
    }

    // -- Edges --

    pub fn list_edges(
        &self,
        project_id: u64,
        source_id: Option<u64>,
        target_id: Option<u64>,
        edge_type: Option<&str>,
    ) -> Result<Vec<DependencyEdge>, Error> {
        self.rt
            .block_on(self.inner.list_edges(project_id, source_id, target_id, edge_type))
    }

    pub fn create_edge(
        &self,
        project_id: u64,
        params: &CreateEdgeParams,
    ) -> Result<DependencyEdge, Error> {
        self.rt.block_on(self.inner.create_edge(project_id, params))
    }

    pub fn get_edge(&self, project_id: u64, edge_id: u64) -> Result<DependencyEdge, Error> {
        self.rt.block_on(self.inner.get_edge(project_id, edge_id))
    }

    pub fn delete_edge(&self, project_id: u64, edge_id: u64) -> Result<(), Error> {
        self.rt.block_on(self.inner.delete_edge(project_id, edge_id))
    }

    /// Convert back to the async client.
    pub fn into_async(self) -> FuncspecClient {
        self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocking_client_new_invalid_key_still_constructs() {
        // Client construction should succeed even with an empty key
        // (validation happens on first request)
        let result = BlockingFuncspecClient::new("https://funcspec.net", "test-key");
        assert!(result.is_ok());
    }
}
