//! Integration tests for FuncspecClient — uses wiremock to mock HTTP.

use funcspec_client::{
    Error, FuncspecClient,
    models::{
        ApiListResponse, ApiResponse, CreateItemParams, ImplementationStatus, ItemFilter, ItemType,
        PaginationMeta, Project, ProjectAttributes, SpecItem, SpecItemAttributes, UpdateItemParams,
    },
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
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
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
        .respond_with(ResponseTemplate::new(404).set_body_json(&body))
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

// -- get_project_stats -------------------------------------------------------

fn make_project_stats_json() -> serde_json::Value {
    json!({
        "data": {
            "total_items": 42,
            "functional_count": 12,
            "technical_count": 30,
            "status_breakdown": {
                "implemented": 28,
                "in_progress": 8,
                "not_started": 6
            },
            "review_coverage": {
                "reviewed_count": 35,
                "total_count": 42,
                "avg_score": 87.2
            },
            "verdict_distribution": {
                "pass": 20,
                "needs_refinement": 12,
                "major_gaps": 3
            },
            "tag_summary": {"auth": 5, "backend": 10},
            "recent_activity": [
                {
                    "item_id": "F-5",
                    "item_title": "AI Operations",
                    "updated_at": "2026-03-20T10:00:00Z",
                    "activity_type": "updated"
                }
            ],
            "last_updated": "2026-03-20T10:00:00Z"
        }
    })
}

#[tokio::test]
async fn get_project_stats_success() {
    let server = setup_server().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/stats"))
        .and(header("X-Api-Key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&make_project_stats_json()))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let stats = client.get_project_stats(1).await.unwrap();
    assert_eq!(stats.total_items, 42);
    assert_eq!(stats.functional_count, 12);
    assert_eq!(stats.technical_count, 30);
    assert_eq!(stats.review_coverage.reviewed_count, 35);
    assert!((stats.review_coverage.avg_score.unwrap() - 87.2).abs() < 1e-9);
    assert_eq!(stats.verdict_distribution.pass, 20);
    assert_eq!(stats.recent_activity.len(), 1);
    assert_eq!(stats.recent_activity[0].item_id, "F-5");
}

#[tokio::test]
async fn get_project_stats_not_found() {
    let server = setup_server().await;
    let body = json!({ "error": "not found" });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/999/stats"))
        .respond_with(ResponseTemplate::new(404).set_body_json(&body))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(&make_usage_stats_json()))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(&make_usage_stats_json()))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
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
        .respond_with(ResponseTemplate::new(201).set_body_json(&body))
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
        .respond_with(ResponseTemplate::new(201).set_body_json(&body))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
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
        .respond_with(ResponseTemplate::new(404).set_body_json(&body))
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
        .respond_with(ResponseTemplate::new(404).set_body_json(&body))
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
        .respond_with(ResponseTemplate::new(404).set_body_json(&body))
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
            "snapshot_id": 1,
            "added": [make_item_json(10, "F-10", "New feature")],
            "removed": [make_item_json(2, "F-2", "Removed feature")],
            "modified": [
                {
                    "before": make_item_json(3, "F-3", "Old title"),
                    "after": make_item_json(3, "F-3", "New title")
                }
            ]
        }
    });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/snapshots/1/diff"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let diff = client.diff_snapshot(1, 1).await.unwrap();
    assert_eq!(diff.snapshot_id, 1);
    assert_eq!(diff.added.len(), 1);
    assert_eq!(diff.added[0].attributes.permalink, "F-10");
    assert_eq!(diff.removed.len(), 1);
    assert_eq!(diff.removed[0].attributes.permalink, "F-2");
    assert_eq!(diff.modified.len(), 1);
    assert_eq!(diff.modified[0].before.attributes.title, "Old title");
    assert_eq!(diff.modified[0].after.attributes.title, "New title");
}

#[tokio::test]
async fn diff_snapshot_empty() {
    let server = setup_server().await;
    let body = json!({
        "data": {
            "snapshot_id": 2,
            "added": [],
            "removed": [],
            "modified": []
        }
    });

    Mock::given(method("GET"))
        .and(path("/api/v1/projects/1/snapshots/2/diff"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&body))
        .mount(&server)
        .await;

    let client = client_for(&server);
    let diff = client.diff_snapshot(1, 2).await.unwrap();
    assert!(diff.added.is_empty());
    assert!(diff.removed.is_empty());
    assert!(diff.modified.is_empty());
}
