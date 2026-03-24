//! Integration tests for FuncspecClient — uses wiremock to mock HTTP.

use funcspec_client::{
    models::{
        ApiListResponse, ApiResponse, CreateItemParams, ItemFilter, ItemType, PaginationMeta,
        Project, ProjectAttributes, SpecItem, SpecItemAttributes, UpdateItemParams,
        ImplementationStatus,
    },
    Error, FuncspecClient,
};
use serde_json::json;
use wiremock::{
    matchers::{header, method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

// -- Helpers -----------------------------------------------------------------

fn make_project_json(id: u64, slug: &str, name: &str) -> serde_json::Value {
    json!({
        "id": id,
        "type": "project",
        "attributes": {
            "name": name,
            "description": null,
            "slug": slug,
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-06-01T00:00:00Z"
        }
    })
}

fn make_item_json(id: u64, permalink: &str, title: &str) -> serde_json::Value {
    json!({
        "id": id,
        "type": "spec_item",
        "attributes": {
            "title": title,
            "description": null,
            "type_of": "functional",
            "state": "active",
            "implementation_status": "not_started",
            "permalink": permalink,
            "url": format!("https://funcspec.net/items/{id}"),
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
    })
}

async fn setup_server() -> MockServer {
    MockServer::start().await
}

fn client_for(server: &MockServer) -> FuncspecClient {
    FuncspecClient::new(&server.uri(), "test-api-key").unwrap()
}

// -- list_projects -----------------------------------------------------------

#[tokio::test]
async fn list_projects_success() {
    let server = setup_server().await;
    let body = json!({
        "data": [make_project_json(1, "my-proj", "My Project")],
        "meta": null
    });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects"))
        .and(header("X-Api-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let projects = client.list_projects().await.unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].attributes.slug, "my-proj");
    assert_eq!(projects[0].attributes.name, "My Project");
}

#[tokio::test]
async fn list_projects_empty() {
    let server = setup_server().await;
    let body = json!({ "data": [], "meta": null });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let projects = client.list_projects().await.unwrap();
    assert!(projects.is_empty());
}

#[tokio::test]
async fn list_projects_401_returns_auth_error() {
    let server = setup_server().await;
    let body = json!({ "error": "invalid token" });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects"))
        .respond_with(ResponseTemplate::new(401).set_body_json(&body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let err = client.list_projects().await.unwrap_err();
    assert!(matches!(err, Error::Auth(_)));
}

#[tokio::test]
async fn list_projects_500_returns_server_error() {
    let server = setup_server().await;
    let body = json!({ "error": "internal server error" });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects"))
        .respond_with(ResponseTemplate::new(500).set_body_json(&body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let err = client.list_projects().await.unwrap_err();
    assert!(matches!(err, Error::Server { status: 500, .. }));
    assert!(err.is_retryable());
}

// -- get_project -------------------------------------------------------------

#[tokio::test]
async fn get_project_success() {
    let server = setup_server().await;
    let body = json!({ "data": make_project_json(42, "test-slug", "Test") });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/test-slug"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let project = client.get_project("test-slug").await.unwrap();
    assert_eq!(project.id, 42);
    assert_eq!(project.attributes.slug, "test-slug");
}

#[tokio::test]
async fn get_project_404() {
    let server = setup_server().await;
    let body = json!({ "error": "not found" });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/no-such"))
        .respond_with(ResponseTemplate::new(404).set_body_json(&body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let err = client.get_project("no-such").await.unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
    assert!(!err.is_retryable());
}

// -- list_items --------------------------------------------------------------

#[tokio::test]
async fn list_items_success_with_meta() {
    let server = setup_server().await;
    let body = json!({
        "data": [
            make_item_json(1, "F-1", "Feature one"),
            make_item_json(2, "F-2", "Feature two")
        ],
        "meta": { "page": 1, "per": 25, "total": 2, "total_pages": 1 }
    });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/10/spec/items"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let filter = ItemFilter::default();
    let (items, meta) = client.list_items(10, &filter).await.unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].attributes.permalink, "F-1");
    let meta = meta.unwrap();
    assert_eq!(meta.total, 2);
}

#[tokio::test]
async fn list_items_filter_sent_as_query_params() {
    let server = setup_server().await;
    let body = json!({ "data": [], "meta": null });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/5/spec/items"))
        .and(query_param("type_of", "functional"))
        .and(query_param("implementation_status", "implemented"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let filter = ItemFilter {
        type_of: Some(ItemType::Functional),
        status: Some(ImplementationStatus::Implemented),
        ..Default::default()
    };
    let (items, _) = client.list_items(5, &filter).await.unwrap();
    assert!(items.is_empty());
}

// -- get_item ----------------------------------------------------------------

#[tokio::test]
async fn get_item_by_permalink() {
    let server = setup_server().await;
    let body = json!({ "data": make_item_json(7, "F-7", "Auth flow") });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/spec/items/F-7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let item = client.get_item(1, "F-7").await.unwrap();
    assert_eq!(item.attributes.permalink, "F-7");
    assert_eq!(item.attributes.title, "Auth flow");
}

// -- create_item -------------------------------------------------------------

#[tokio::test]
async fn create_item_success() {
    let server = setup_server().await;
    let body = json!({ "data": make_item_json(99, "F-99", "New feature") });

    Mock::given(method("POST"))
        .and(path("/api/v1/projects/1/spec/items"))
        .respond_with(ResponseTemplate::new(201).set_body_json(&body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let params = CreateItemParams {
        title: "New feature".into(),
        type_of: "functional".into(),
        ..Default::default()
    };
    let item = client.create_item(1, &params).await.unwrap();
    assert_eq!(item.attributes.permalink, "F-99");
}

#[tokio::test]
async fn create_item_422_validation_error() {
    let server = setup_server().await;
    let body = json!({ "error": "title is required" });

    Mock::given(method("POST"))
        .and(path("/api/v1/projects/1/spec/items"))
        .respond_with(ResponseTemplate::new(422).set_body_json(&body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let params = CreateItemParams::default();
    let err = client.create_item(1, &params).await.unwrap_err();
    assert!(matches!(err, Error::Validation(_)));
    assert!(!err.is_retryable());
}

// -- update_item -------------------------------------------------------------

#[tokio::test]
async fn update_item_success() {
    let server = setup_server().await;
    let mut item_json = make_item_json(5, "F-5", "Updated title");
    item_json["attributes"]["title"] = serde_json::Value::String("Updated title".into());
    let body = json!({ "data": item_json });

    Mock::given(method("PATCH"))
        .and(path("/api/v1/projects/1/spec/items/5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let params = UpdateItemParams {
        title: Some("Updated title".into()),
        ..Default::default()
    };
    let item = client.update_item(1, 5, &params).await.unwrap();
    assert_eq!(item.attributes.title, "Updated title");
}

// -- delete_item -------------------------------------------------------------

#[tokio::test]
async fn delete_item_success() {
    let server = setup_server().await;

    Mock::given(method("DELETE"))
        .and(path("/api/v1/projects/1/spec/items/3"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = client_for(&server);
    client.delete_item(1, 3).await.unwrap();
}

// -- rate limit / 429 --------------------------------------------------------

#[tokio::test]
async fn rate_limited_error_classification() {
    let server = setup_server().await;
    let body = json!({ "error": "too many requests" });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects"))
        // Return 429 on all attempts (mock fires multiple times)
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "1")
                .set_body_json(&body),
        )
        .expect(1..=4) // up to max_retries + initial attempt
        .mount(&server)
        .await;

    let client = client_for(&server);
    let err = client.list_projects().await.unwrap_err();
    // After exhausting retries, returns 429 or falls through
    // The important thing is we get a rate-limited error
    assert!(
        matches!(err, Error::RateLimited { .. }),
        "Expected RateLimited, got: {err:?}"
    );
}

// -- pagination --------------------------------------------------------------

#[tokio::test]
async fn list_projects_paged_returns_metadata() {
    let server = setup_server().await;
    let body = json!({
        "data": [
            make_project_json(1, "proj-a", "Project A"),
            make_project_json(2, "proj-b", "Project B")
        ],
        "meta": { "page": 1, "per": 2, "total": 10, "total_pages": 5 }
    });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let paged = client.list_projects_paged(1, 2).await.unwrap();
    assert_eq!(paged.page, 1);
    assert_eq!(paged.total_count, 10);
    assert_eq!(paged.total_pages, 5);
    assert!(paged.has_next_page());
    assert_eq!(paged.data.len(), 2);
}

// -- error helpers -----------------------------------------------------------

#[tokio::test]
async fn forbidden_error_from_403() {
    let server = setup_server().await;
    let body = json!({ "error": "access denied" });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects"))
        .respond_with(ResponseTemplate::new(403).set_body_json(&body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let err = client.list_projects().await.unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
}
