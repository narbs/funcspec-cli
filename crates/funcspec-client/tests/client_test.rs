//! Integration tests for FuncspecClient — uses wiremock to mock HTTP.

use funcspec_client::{
    Error, FuncspecClient,
    models::{CreateItemParams, ImplementationStatus, ItemFilter, ItemType, UpdateItemParams},
};
use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
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
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
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
        .respond_with(ResponseTemplate::new(401).set_body_json(body))
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
        .respond_with(ResponseTemplate::new(500).set_body_json(body))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
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
        .respond_with(ResponseTemplate::new(404).set_body_json(body))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
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

    // get_item now sends the permalink directly: GET /projects/1/spec/items/F-7
    let show_body = json!({ "data": make_item_json(7, "F-7", "Auth flow") });
    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/spec/items/F-7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(show_body))
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
        .respond_with(ResponseTemplate::new(201).set_body_json(body))
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
        .respond_with(ResponseTemplate::new(422).set_body_json(body))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
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

// -- search_items ------------------------------------------------------------

#[tokio::test]
async fn search_items_sends_q_param() {
    let server = setup_server().await;
    let body = json!({
        "data": [make_item_json(1, "F-1", "Auth feature")],
        "meta": { "page": 1, "per": 25, "total": 1, "total_pages": 1 }
    });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/spec/items"))
        .and(query_param("q", "auth"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let filter = ItemFilter::default();
    let result = client.search_items(1, "auth", &filter).await.unwrap();
    assert_eq!(result.data.len(), 1);
    assert_eq!(result.data[0].attributes.permalink, "F-1");
    assert_eq!(result.total_count, 1);
}

#[tokio::test]
async fn search_items_with_type_filter() {
    let server = setup_server().await;
    let body = json!({ "data": [], "meta": null });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/spec/items"))
        .and(query_param("q", "login"))
        .and(query_param("type_of", "technical"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let filter = ItemFilter {
        type_of: Some(ItemType::Technical),
        ..Default::default()
    };
    let result = client.search_items(1, "login", &filter).await.unwrap();
    assert!(result.data.is_empty());
}

#[tokio::test]
async fn search_items_overrides_filter_q() {
    // If filter already has a q, search_items should replace it with the query arg
    let server = setup_server().await;
    let body = json!({ "data": [], "meta": null });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/spec/items"))
        .and(query_param("q", "override"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let filter = ItemFilter {
        q: Some("ignored".into()), // this should be replaced
        ..Default::default()
    };
    let result = client.search_items(1, "override", &filter).await.unwrap();
    assert!(result.data.is_empty());
}

#[tokio::test]
async fn list_items_with_sort_param() {
    let server = setup_server().await;
    let body = json!({ "data": [], "meta": null });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/spec/items"))
        .and(query_param("sort", "score"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let filter = ItemFilter {
        sort: Some("score".into()),
        ..Default::default()
    };
    let (items, _) = client.list_items(1, &filter).await.unwrap();
    assert!(items.is_empty());
}

#[tokio::test]
async fn search_items_404_returns_not_found() {
    let server = setup_server().await;
    let body = json!({ "error": "project not found" });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/999/spec/items"))
        .respond_with(ResponseTemplate::new(404).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let filter = ItemFilter::default();
    let err = client
        .search_items(999, "anything", &filter)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
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
                .set_body_json(body),
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
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
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

// -- resolve_item_id ---------------------------------------------------------

#[tokio::test]
async fn resolve_item_id_numeric_string_returns_without_http() {
    // A numeric string should be returned directly without any HTTP call
    let server = setup_server().await;
    // No mock mounted — any HTTP call would panic
    let client = client_for(&server);
    let id = client.resolve_item_id(1, "42").await.unwrap();
    assert_eq!(id, 42);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn resolve_item_id_permalink_fetches_item_and_returns_numeric_id() {
    // T-101 / T-103 root cause: a permalink like "F-396" must be resolved to its
    // numeric ID via GET /projects/:id/spec/items/:permalink, not silently dropped.
    let server = setup_server().await;
    let body = json!({ "data": make_item_json(396, "F-396", "Some feature") });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/spec/items/F-396"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let id = client.resolve_item_id(1, "F-396").await.unwrap();
    assert_eq!(id, 396);
}

#[tokio::test]
async fn resolve_item_id_not_found_returns_error() {
    let server = setup_server().await;
    let body = json!({ "error": "not found" });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/spec/items/F-999"))
        .respond_with(ResponseTemplate::new(404).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let err = client.resolve_item_id(1, "F-999").await.unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
}

// -- T-103: list_items parent_id filter -------------------------------------

#[tokio::test]
async fn list_items_parent_id_sent_as_query_param() {
    // T-103: when parent_id is set in ItemFilter it must reach the API as a
    // query parameter. Previously a permalink-derived parent_id was silently
    // dropped before reaching the client; this test exercises the client side.
    let server = setup_server().await;
    let body = json!({
        "data": [make_item_json(10, "T-10", "Child item")],
        "meta": null
    });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/spec/items"))
        .and(query_param("parent_id", "396"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let filter = ItemFilter {
        parent_id: Some(396),
        ..Default::default()
    };
    let (items, _) = client.list_items(1, &filter).await.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].attributes.permalink, "T-10");
}

// -- T-101: create_item parent_id in POST body ------------------------------

#[tokio::test]
async fn create_item_with_parent_id_sends_parent_id_in_body() {
    // T-101: parent_id must be included in the POST payload, not omitted.
    let server = setup_server().await;
    let mut item = make_item_json(55, "T-55", "Child spec");
    item["attributes"]["parent_id"] = json!(12);
    let body = json!({ "data": item });

    Mock::given(method("POST"))
        .and(path("/api/v1/projects/1/spec/items"))
        .respond_with(ResponseTemplate::new(201).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let params = CreateItemParams {
        title: "Child spec".into(),
        type_of: "technical".into(),
        parent_id: Some(12),
        ..Default::default()
    };
    client.create_item(1, &params).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let sent: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(sent["parent_id"], json!(12));
}

#[tokio::test]
async fn create_item_without_parent_omits_parent_id_from_body() {
    // parent_id must be absent (not null) when not provided
    let server = setup_server().await;
    let body = json!({ "data": make_item_json(56, "F-56", "Top-level item") });

    Mock::given(method("POST"))
        .and(path("/api/v1/projects/1/spec/items"))
        .respond_with(ResponseTemplate::new(201).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let params = CreateItemParams {
        title: "Top-level item".into(),
        type_of: "functional".into(),
        ..Default::default()
    };
    client.create_item(1, &params).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let sent: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert!(!sent.as_object().unwrap().contains_key("parent_id"));
}

// -- T-102: update_item parent_id in PATCH body -----------------------------

#[tokio::test]
async fn update_item_with_parent_sends_numeric_parent_id() {
    // T-102: --parent flag must serialize as a JSON number in the PATCH body.
    let server = setup_server().await;
    let body = json!({ "data": make_item_json(5, "F-5", "Item") });

    Mock::given(method("PATCH"))
        .and(path("/api/v1/projects/1/spec/items/5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let params = UpdateItemParams {
        parent_id: Some(serde_json::json!(42u64)),
        ..Default::default()
    };
    client.update_item(1, 5, &params).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let sent: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(sent["parent_id"], json!(42));
}

#[tokio::test]
async fn update_item_with_no_parent_sends_null_parent_id() {
    // T-102: --no-parent flag must serialize parent_id as JSON null.
    let server = setup_server().await;
    let body = json!({ "data": make_item_json(5, "F-5", "Item") });

    Mock::given(method("PATCH"))
        .and(path("/api/v1/projects/1/spec/items/5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let params = UpdateItemParams {
        parent_id: Some(serde_json::Value::Null),
        ..Default::default()
    };
    client.update_item(1, 5, &params).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let sent: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert!(sent.as_object().unwrap().contains_key("parent_id"));
    assert!(sent["parent_id"].is_null());
}

#[tokio::test]
async fn update_item_without_parent_flag_omits_parent_id() {
    // When neither --parent nor --no-parent is given, parent_id must be absent.
    let server = setup_server().await;
    let body = json!({ "data": make_item_json(5, "F-5", "Item") });

    Mock::given(method("PATCH"))
        .and(path("/api/v1/projects/1/spec/items/5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let params = UpdateItemParams {
        title: Some("New title".into()),
        ..Default::default()
    };
    client.update_item(1, 5, &params).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let sent: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert!(!sent.as_object().unwrap().contains_key("parent_id"));
}

// -- error helpers -----------------------------------------------------------

#[tokio::test]
async fn forbidden_error_from_403() {
    let server = setup_server().await;
    let body = json!({ "error": "access denied" });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects"))
        .respond_with(ResponseTemplate::new(403).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let err = client.list_projects().await.unwrap_err();
    assert!(matches!(err, Error::Forbidden(_)));
}

// -- get_project_stats -------------------------------------------------------

fn make_project_stats_json() -> serde_json::Value {
    json!({
        "data": {
            "type": "project_stats",
            "spec_items": {
                "total": 42,
                "by_type": {"functional": 12, "technical": 30},
                "by_state": {"inbox": 42},
                "by_implementation": {"implemented": 28, "in_progress": 8, "not_started": 6}
            },
            "reviews": {
                "tech_reviewed": 30,
                "tech_unreviewed": 0,
                "func_reviewed": 5,
                "func_unreviewed": 7,
                "avg_tech_score": 87.2,
                "avg_func_score": null,
                "by_verdict": {"pass": 20, "needs_refinement": 12, "major_gaps": 3}
            },
            "coverage": {
                "functional_with_tech": 5,
                "functional_without_tech": 7
            },
            "recent_activity": {
                "items_updated_24h": 2,
                "reviews_24h": 3,
                "agent_runs_24h": 0
            }
        }
    })
}

#[tokio::test]
async fn get_project_stats_success() {
    let server = setup_server().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/stats"))
        .and(header("X-Api-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(make_project_stats_json()))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let stats = client.get_project_stats(1).await.unwrap();
    assert_eq!(stats.spec_items.total, 42);
    assert_eq!(stats.spec_items.by_type.get("functional"), Some(&12u32));
    assert_eq!(stats.spec_items.by_type.get("technical"), Some(&30u32));
    assert_eq!(stats.reviews.tech_reviewed, 30);
    assert!((stats.reviews.avg_tech_score.unwrap() - 87.2).abs() < 1e-9);
    assert_eq!(stats.reviews.by_verdict.get("pass"), Some(&20u32));
    assert_eq!(stats.recent_activity.items_updated_24h, 2);
    assert_eq!(stats.recent_activity.reviews_24h, 3);
}

#[tokio::test]
async fn get_project_stats_not_found() {
    let server = setup_server().await;
    let body = json!({ "error": "not found" });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/999/stats"))
        .respond_with(ResponseTemplate::new(404).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let err = client.get_project_stats(999).await.unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
}

// -- get_usage_stats ---------------------------------------------------------

fn make_usage_stats_json() -> serde_json::Value {
    json!({
        "data": {
            "month": "2026-03",
            "total_tokens": 45200,
            "estimated_cost": 0.12,
            "breakdown_by_operation": {
                "review": {"tokens": 30000, "cost": 0.08},
                "analysis": {"tokens": 15200, "cost": 0.04}
            },
            "last_updated": "2026-03-24T00:00:00Z"
        }
    })
}

#[tokio::test]
async fn get_usage_stats_success() {
    let server = setup_server().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/usage"))
        .and(header("X-Api-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(make_usage_stats_json()))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let stats = client.get_usage_stats(1, None).await.unwrap();
    assert_eq!(stats.month, "2026-03");
    assert_eq!(stats.total_tokens, 45200);
    assert!((stats.estimated_cost - 0.12).abs() < 1e-9);
    assert_eq!(
        stats.breakdown_by_operation.get("review").map(|u| u.tokens),
        Some(30000)
    );
}

#[tokio::test]
async fn get_usage_stats_with_month_param() {
    let server = setup_server().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/usage"))
        .and(query_param("month", "2026-02"))
        .respond_with(ResponseTemplate::new(200).set_body_json(make_usage_stats_json()))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let stats = client.get_usage_stats(1, Some("2026-02")).await.unwrap();
    assert_eq!(stats.total_tokens, 45200);
}

// -- get_usage_logs ----------------------------------------------------------

#[tokio::test]
async fn get_usage_logs_success() {
    use funcspec_client::models::UsageFilter;
    let server = setup_server().await;
    let body = json!({
        "data": [],
        "meta": {"page": 1, "per": 25, "total": 0, "total_pages": 0}
    });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/usage/logs"))
        .and(header("X-Api-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let result = client
        .get_usage_logs(1, &UsageFilter::default())
        .await
        .unwrap();
    assert!(result.data.is_empty());
    assert_eq!(result.total_count, 0);
}

// -- snapshots ---------------------------------------------------------------

fn make_snapshot_json(id: u64, name: &str) -> serde_json::Value {
    json!({
        "id": id,
        "type": "snapshot",
        "attributes": {
            "project_id": 1,
            "name": name,
            "description": null,
            "spec_items": [make_item_json(1, "F-1", "Feature one")],
            "created_at": "2024-06-01T00:00:00Z"
        }
    })
}

#[tokio::test]
async fn list_snapshots_success() {
    let server = setup_server().await;
    let body = json!({
        "data": [make_snapshot_json(1, "pre-v2"), make_snapshot_json(2, "baseline")],
        "meta": null
    });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/10/snapshots"))
        .and(header("X-Api-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let snapshots = client.list_snapshots(10).await.unwrap();
    assert_eq!(snapshots.len(), 2);
    assert_eq!(snapshots[0].attributes.name, "pre-v2");
    assert_eq!(snapshots[1].attributes.name, "baseline");
}

#[tokio::test]
async fn list_snapshots_empty() {
    let server = setup_server().await;
    let body = json!({ "data": [], "meta": null });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/10/snapshots"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let snapshots = client.list_snapshots(10).await.unwrap();
    assert!(snapshots.is_empty());
}

#[tokio::test]
async fn create_snapshot_success() {
    use funcspec_client::models::CreateSnapshotParams;
    let server = setup_server().await;
    let body = json!({ "data": make_snapshot_json(42, "pre-v2-refactor") });

    Mock::given(method("POST"))
        .and(path("/api/v1/projects/1/snapshots"))
        .respond_with(ResponseTemplate::new(201).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let params = CreateSnapshotParams {
        name: "pre-v2-refactor".into(),
        description: None,
    };
    let snapshot = client.create_snapshot(1, &params).await.unwrap();
    assert_eq!(snapshot.id, 42);
    assert_eq!(snapshot.attributes.name, "pre-v2-refactor");
    assert_eq!(snapshot.attributes.spec_items.len(), 1);
}

#[tokio::test]
async fn create_snapshot_with_description() {
    use funcspec_client::models::CreateSnapshotParams;
    let server = setup_server().await;
    let mut snap = make_snapshot_json(5, "with-desc");
    snap["attributes"]["description"] = serde_json::Value::String("A description".into());
    let body = json!({ "data": snap });

    Mock::given(method("POST"))
        .and(path("/api/v1/projects/1/snapshots"))
        .respond_with(ResponseTemplate::new(201).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let params = CreateSnapshotParams {
        name: "with-desc".into(),
        description: Some("A description".into()),
    };
    let snapshot = client.create_snapshot(1, &params).await.unwrap();
    assert_eq!(
        snapshot.attributes.description.as_deref(),
        Some("A description")
    );
}

#[tokio::test]
async fn get_snapshot_success() {
    let server = setup_server().await;
    let body = json!({ "data": make_snapshot_json(7, "my-snap") });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/snapshots/7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let snapshot = client.get_snapshot(1, 7).await.unwrap();
    assert_eq!(snapshot.id, 7);
    assert_eq!(snapshot.attributes.name, "my-snap");
}

#[tokio::test]
async fn get_snapshot_404() {
    let server = setup_server().await;
    let body = json!({ "error": "not found" });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/snapshots/999"))
        .respond_with(ResponseTemplate::new(404).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let err = client.get_snapshot(1, 999).await.unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
}

#[tokio::test]
async fn restore_snapshot_success() {
    let server = setup_server().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/projects/1/snapshots/3/restore"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let client = client_for(&server);
    client.restore_snapshot(1, 3).await.unwrap();
}

#[tokio::test]
async fn restore_snapshot_not_found() {
    let server = setup_server().await;
    let body = json!({ "error": "snapshot not found" });

    Mock::given(method("POST"))
        .and(path("/api/v1/projects/1/snapshots/999/restore"))
        .respond_with(ResponseTemplate::new(404).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let err = client.restore_snapshot(1, 999).await.unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
}

#[tokio::test]
async fn delete_snapshot_success() {
    let server = setup_server().await;

    Mock::given(method("DELETE"))
        .and(path("/api/v1/projects/1/snapshots/5"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = client_for(&server);
    client.delete_snapshot(1, 5).await.unwrap();
}

#[tokio::test]
async fn delete_snapshot_not_found() {
    let server = setup_server().await;
    let body = json!({ "error": "not found" });

    Mock::given(method("DELETE"))
        .and(path("/api/v1/projects/1/snapshots/999"))
        .respond_with(ResponseTemplate::new(404).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let err = client.delete_snapshot(1, 999).await.unwrap_err();
    assert!(matches!(err, Error::NotFound(_)));
}

#[tokio::test]
async fn diff_snapshot_success() {
    let server = setup_server().await;
    let body = json!({
        "data": {
            "spec_items": {
                "added": [{"id": 10, "permalink": "F-10", "title": "New feature"}],
                "removed": [{"id": 2, "permalink": "F-2", "title": "Removed feature"}],
                "modified": [
                    {
                        "permalink": "F-3",
                        "before": {"id": 3, "permalink": "F-3", "title": "Old title"},
                        "after":  {"id": 3, "permalink": "F-3", "title": "New title"},
                        "changes": {"title": {"before": "Old title", "after": "New title"}}
                    }
                ]
            },
            "edges": {"added": [], "removed": []},
            "summary": {
                "items_added": 1, "items_removed": 1, "items_modified": 1,
                "edges_added": 0, "edges_removed": 0
            }
        }
    });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/snapshots/1/diff"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let diff = client.diff_snapshot(1, 1).await.unwrap();
    assert_eq!(diff.summary.items_added, 1);
    assert_eq!(diff.summary.items_removed, 1);
    assert_eq!(diff.summary.items_modified, 1);
    assert_eq!(diff.spec_items.added.len(), 1);
    assert_eq!(diff.spec_items.added[0]["permalink"], "F-10");
    assert_eq!(diff.spec_items.removed.len(), 1);
    assert_eq!(diff.spec_items.removed[0]["permalink"], "F-2");
    assert_eq!(diff.spec_items.modified.len(), 1);
    assert_eq!(diff.spec_items.modified[0].permalink, "F-3");
    assert_eq!(diff.spec_items.modified[0].changes["title"]["before"], "Old title");
    assert_eq!(diff.spec_items.modified[0].changes["title"]["after"], "New title");
}

#[tokio::test]
async fn diff_snapshot_empty() {
    let server = setup_server().await;
    let body = json!({
        "data": {
            "spec_items": {"added": [], "removed": [], "modified": []},
            "edges": {"added": [], "removed": []},
            "summary": {
                "items_added": 0, "items_removed": 0, "items_modified": 0,
                "edges_added": 0, "edges_removed": 0
            }
        }
    });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/snapshots/2/diff"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let diff = client.diff_snapshot(1, 2).await.unwrap();
    assert!(diff.spec_items.added.is_empty());
    assert!(diff.spec_items.removed.is_empty());
    assert!(diff.spec_items.modified.is_empty());
    assert_eq!(diff.summary.items_added, 0);
}
