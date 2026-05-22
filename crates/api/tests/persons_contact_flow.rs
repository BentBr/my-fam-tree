//! Integration coverage for the persons `email`/`phone`/postal-address
//! columns plus the "email synced from linked user" rule.
//!
//! Split out of `persons_flow.rs` so each binary stays under the 500-line
//! test-file cap. Reuses the same `common` scaffolding (ephemeral
//! Postgres + Redis testcontainers, magic-link sign-in, etc.).

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
use common::{create_family, ephemeral_stack, sign_in};
use my_family_api::build_app;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn persons_contact_fields_round_trip() {
    // Create + PATCH the new contact columns and assert the PersonView
    // mirrors them on every read path (POST, GET, PATCH).
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "contact-owner@example.com").await;
    let (access, family_id) = create_family(&app, &access, "ContactFam").await;

    // POST with every contact field — values come back unchanged.
    let create_body = serde_json::json!({
        "given_name": "Greta",
        "family_name": "Schmidt",
        "email": "greta@example.de",
        "phone": "+49 30 1234 5678",
        "street": "Friedrich-Ebert-Allee",
        "house_number": "25b",
        "zip": "10115",
        "city": "Berlin",
        "country": "Deutschland",
    });
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(create_body)
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["email"], "greta@example.de");
    assert_eq!(body["data"]["phone"], "+49 30 1234 5678");
    assert_eq!(body["data"]["street"], "Friedrich-Ebert-Allee");
    assert_eq!(body["data"]["house_number"], "25b");
    assert_eq!(body["data"]["zip"], "10115");
    assert_eq!(body["data"]["city"], "Berlin");
    assert_eq!(body["data"]["country"], "Deutschland");
    let person_id = body["data"]["id"].as_str().expect("person id").to_string();

    // PATCH only `phone` + `city` — the other fields keep their values
    // (merge-not-replace, which is the documented PATCH semantics).
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({ "phone": "+49 30 9999", "city": "Hamburg" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["phone"], "+49 30 9999");
    assert_eq!(body["data"]["city"], "Hamburg");
    // Untouched fields survive the merge.
    assert_eq!(body["data"]["street"], "Friedrich-Ebert-Allee");
    assert_eq!(body["data"]["zip"], "10115");
    assert_eq!(body["data"]["country"], "Deutschland");

    // GET /persons/{id} confirms the merged state landed.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["phone"], "+49 30 9999");
    assert_eq!(body["data"]["city"], "Hamburg");

    // Garbage email → 422 validation.email_invalid.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id))
        .set_json(serde_json::json!({ "email": "not-an-email" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.email_invalid");
    assert_eq!(body["fields"][0]["path"], "/email");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn persons_email_synced_from_linked_user_overrides_body() {
    // When the (post-change) state has `linked_user_id` set, the API
    // ignores any `email` in the body and writes the column from
    // `users.email`. This holds for both POST and PATCH.
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let owner_email = "linked-owner@example.com";
    let (access, _r) = sign_in(&stack, &app, owner_email).await;
    let (access, family_id) = create_family(&app, &access, "LinkedFam").await;

    // The sign-in flow created a row in `users` for owner_email. Look it
    // up so we can set `linked_user_id`.
    let owner_user =
        stack.state.users.find_by_email(owner_email).await.expect("lookup").expect("owner user");

    // POST with a deliberately bogus `email`. The server must overwrite
    // it from users.email = linked-owner@example.com.
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "given_name": "Owner",
            "linked_user_id": owner_user.id.into_uuid().to_string(),
            "email": "trojan@evil.example",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["email"], owner_email);
    let person_id = body["data"]["id"].as_str().expect("person id").to_string();

    // PATCH the same row with another bogus email — still overridden.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({ "email": "still-trojan@evil.example" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["email"], owner_email);

    // GET confirms the persisted column matches the linked user's email,
    // not whatever the most recent body said.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["email"], owner_email);
}
