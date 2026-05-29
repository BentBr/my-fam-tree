//! Integration coverage for `GET /api/v1/health` against a real Postgres
//! (testcontainers): confirms the envelope, the cargo version, the request-id
//! echo, and the DB reachability probe (`db_ok` + measured `db_latency_ms`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::future_not_send,
    clippy::shadow_unrelated,
    clippy::shadow_reuse,
    clippy::shadow_same,
    reason = "test code: testcontainers + assertion helpers may panic and aren't Send-bounded; shadowing matches the existing integration-test convention"
)]

mod common;

use actix_web::test;
use common::ephemeral_stack;
use my_fam_tree_api::build_app;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_reports_ok_version_and_reachable_db() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/health")
        .insert_header(("x-request-id", "rid-test"))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200, "health is always 200");

    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["status"], "ok");
    assert!(body["data"]["version"].is_string(), "version present (cargo version)");
    assert_eq!(body["data"]["db_ok"], true, "migrated DB is reachable");
    assert!(body["data"]["db_latency_ms"].is_number(), "latency measured");
    assert_eq!(body["meta"]["request_id"], "rid-test", "request id echoed to meta");
}
