//! Integration coverage for `GET`/`PUT /api/v1/reminder-preferences`.
//!
//! Exercises the per-user settings round-trip against the real Postgres stack
//! via testcontainers: defaults on first read, upsert + read-back, idempotent
//! re-save, and the `lead_days` range validation (422).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::future_not_send,
    clippy::shadow_unrelated,
    clippy::shadow_reuse,
    clippy::shadow_same,
    reason = "test code: testcontainers + assertion helpers may panic and aren't Send-bounded; shadowing is the convention used by the existing API integration tests"
)]

mod common;

use actix_web::cookie::Cookie;
use actix_web::test;
use common::{ephemeral_stack, sign_in};
use my_fam_tree_api::build_app;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reminder_prefs_defaults_then_upsert_round_trip() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "reminders-prefs@example.com").await;

    // First GET — never saved ⇒ built-in defaults (emails OFF, both kinds on,
    // all people, 7 days ahead).
    let req = test::TestRequest::get()
        .uri("/api/v1/reminder-preferences")
        .cookie(Cookie::new("access", access.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["emails_enabled"], false);
    assert_eq!(body["data"]["remind_birthdays"], true);
    assert_eq!(body["data"]["remind_anniversaries"], true);
    assert_eq!(body["data"]["favourites_only"], false);
    assert_eq!(body["data"]["lead_days"], 7);

    // PUT new settings.
    let req = test::TestRequest::put()
        .uri("/api/v1/reminder-preferences")
        .cookie(Cookie::new("access", access.clone()))
        .set_json(serde_json::json!({
            "emails_enabled": true,
            "remind_birthdays": true,
            "remind_anniversaries": false,
            "favourites_only": true,
            "lead_days": 3,
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["emails_enabled"], true);
    assert_eq!(body["data"]["lead_days"], 3);

    // GET reflects the saved settings.
    let req = test::TestRequest::get()
        .uri("/api/v1/reminder-preferences")
        .cookie(Cookie::new("access", access.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["emails_enabled"], true);
    assert_eq!(body["data"]["remind_anniversaries"], false);
    assert_eq!(body["data"]["favourites_only"], true);
    assert_eq!(body["data"]["lead_days"], 3);

    // Idempotent re-save (upsert on the same user) — still 200, new values win.
    let req = test::TestRequest::put()
        .uri("/api/v1/reminder-preferences")
        .cookie(Cookie::new("access", access.clone()))
        .set_json(serde_json::json!({
            "emails_enabled": false,
            "remind_birthdays": false,
            "remind_anniversaries": true,
            "favourites_only": false,
            "lead_days": 0,
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["lead_days"], 0);
    assert_eq!(body["data"]["emails_enabled"], false);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reminder_prefs_rejects_out_of_range_lead_days() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "reminders-range@example.com").await;

    let req = test::TestRequest::put()
        .uri("/api/v1/reminder-preferences")
        .cookie(Cookie::new("access", access.clone()))
        .set_json(serde_json::json!({
            "emails_enabled": true,
            "remind_birthdays": true,
            "remind_anniversaries": true,
            "favourites_only": false,
            "lead_days": 22,
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422, "lead_days 22 is out of the 0..=21 range");
    let body: serde_json::Value = test::read_body_json(res).await;
    let fields = body["fields"].as_array().expect("violation list");
    assert!(
        fields.iter().any(|f| f["path"] == "/lead_days"),
        "expected a /lead_days violation: {body:?}"
    );
}
