//! Integration coverage for `GET /api/v1/health` against a real Postgres
//! (testcontainers): confirms the envelope, the cargo version, the request-id
//! echo, the DB reachability probe (`db_ok` + measured `db_latency_ms`), and
//! the worker leader-lease check (`worker_ok` — false when no worker is
//! running, which is the expected state under the test harness).

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
    assert!(body["data"]["db_latency_ms"].is_number(), "DB latency measured");
    // worker_ok is a Redis EXISTS check on `<prefix>reminder:leader`.
    // No worker runs against the test container, so the lease key is
    // absent and we expect `false` here — the SHAPE is the contract,
    // the value is incidental.
    assert!(body["data"]["worker_ok"].is_boolean(), "worker_ok present and boolean");
    assert_eq!(body["data"]["worker_ok"], false, "no worker → lease absent → worker_ok=false");
    // Server-side total handler duration. Always present (f64 ms with
    // sub-millisecond precision — warm-pool pings round to 0 with
    // integer ms), always >= db_latency_ms (the DB probe runs inside
    // the handler timer).
    let server = body["data"]["server_duration_ms"]
        .as_f64()
        .expect("server_duration_ms is a number in the response");
    let db = body["data"]["db_latency_ms"].as_f64().expect("db_latency_ms is a number");
    assert!(server >= db, "server_duration_ms ({server}) must be >= db_latency_ms ({db})");
    assert_eq!(body["meta"]["request_id"], "rid-test", "request id echoed to meta");
}
