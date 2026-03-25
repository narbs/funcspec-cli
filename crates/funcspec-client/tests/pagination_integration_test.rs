//! Integration tests for pagination helpers that require HTTP mocking.

use funcspec_client::{
    Error, FuncspecClient, collect_all_pages,
    models::{PaginationMeta, Project},
    pagination::PagedResponse,
    stream_all_pages,
};
use futures::StreamExt;
use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

fn make_project_json(id: u64, slug: &str) -> serde_json::Value {
    json!({
        "id": id,
        "type": "project",
        "attributes": {
            "name": format!("Project {slug}"),
            "description": null,
            "slug": slug,
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        }
    })
}

#[tokio::test]
async fn stream_projects_across_multiple_pages() {
    let server = MockServer::start().await;
    let base_url = server.uri();

    // Page 1
    Mock::given(method("GET"))
        .and(path("/api/v1/projects"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&json!({
            "data": [make_project_json(1, "proj-1"), make_project_json(2, "proj-2")],
            "meta": {"page": 1, "per": 2, "total": 4, "total_pages": 2}
        })))
        .mount(&server)
        .await;

    // Page 2
    Mock::given(method("GET"))
        .and(path("/api/v1/projects"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&json!({
            "data": [make_project_json(3, "proj-3"), make_project_json(4, "proj-4")],
            "meta": {"page": 2, "per": 2, "total": 4, "total_pages": 2}
        })))
        .mount(&server)
        .await;

    let client = FuncspecClient::new(&base_url, "key").unwrap();

    let stream = stream_all_pages::<Project, _, _>(2, move |page, per| {
        let c = client.clone();
        async move { c.list_projects_paged(page, per).await }
    });

    let projects: Vec<Project> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(projects.len(), 4);
    assert_eq!(projects[0].attributes.slug, "proj-1");
    assert_eq!(projects[3].attributes.slug, "proj-4");
}

#[tokio::test]
async fn collect_all_pages_single_request() {
    let server = MockServer::start().await;
    let base_url = server.uri();

    Mock::given(method("GET"))
        .and(path("/api/v1/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&json!({
            "data": [make_project_json(1, "only-proj")],
            "meta": {"page": 1, "per": 50, "total": 1, "total_pages": 1}
        })))
        .mount(&server)
        .await;

    let client = FuncspecClient::new(&base_url, "key").unwrap();

    let all = collect_all_pages::<Project, _, _>(50, move |page, per| {
        let c = client.clone();
        async move { c.list_projects_paged(page, per).await }
    })
    .await
    .unwrap();

    assert_eq!(all.len(), 1);
    assert_eq!(all[0].attributes.slug, "only-proj");
}
