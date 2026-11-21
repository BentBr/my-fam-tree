//! Security-audit regressions for input validation: `deny_unknown_fields`
//! DTOs (`a420ccd`), the per-user `/users/me/email-change` rate cap
//! (`964c299`) and the free-text / family-name length + CR/LF caps
//! (`cf0c9b2`). See `security_regressions.rs` for the audit overview.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::future_not_send,
    clippy::shadow_unrelated,
    clippy::shadow_reuse,
    clippy::shadow_same,
    clippy::too_many_lines,
    reason = "test code: testcontainers + assertion helpers may panic and aren't Send-bounded; shadowing matches the existing flow tests"
)]

mod common;

use actix_web::cookie::Cookie;
use actix_web::test;
use common::{create_family, ephemeral_stack, sign_in};
use my_fam_tree_api::build_app;

// ---------------------------------------------------------------------------
// INFO: DTO deny_unknown_fields (a420ccd)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn persons_create_rejects_unknown_field() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let (access, _r) = sign_in(&stack, &app, "audit-deny@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Deny").await;

    // Body includes a typo'd field — `linkedUserId` (camelCase) instead of
    // `linked_user_id`. With `deny_unknown_fields` this surfaces as a 400 /
    // 422 deserialise failure instead of silently dropping the field.
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id))
        .set_json(serde_json::json!({
            "given_name": "Typo",
            "linkedUserId": "00000000-0000-0000-0000-000000000000",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    // Actix-web returns 400 for JSON deserialise errors before the handler
    // body runs; the 422 path is for handler-level Validation errors.
    let status = res.status().as_u16();
    assert!(
        status == 400 || status == 422,
        "unknown field should be rejected with 400 or 422, got {status}",
    );
}

// ---------------------------------------------------------------------------
// LOW: /users/me/email-change has a per-user rate cap (964c299)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn email_change_rate_caps_at_5_per_hour_per_user() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let (access, _r) = sign_in(&stack, &app, "audit-ec@example.com").await;

    // First five attempts pass the rate gate (they may 4xx on email
    // already-in-use, but they shouldn't be 429). We don't care about the
    // status here — the regression is "rate gate eventually fires".
    for i in 0..5_u8 {
        let req = test::TestRequest::post()
            .uri("/api/v1/users/me/email-change")
            .cookie(Cookie::new("access", access.clone()))
            .set_json(
                serde_json::json!({ "new_email": format!("audit-ec-target-{i}@example.com") }),
            )
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_ne!(res.status(), 429, "request {i} should not be rate-limited yet (cap is 5/hour)");
    }

    // Sixth attempt MUST 429 — rate gate's exceeded.
    let req = test::TestRequest::post()
        .uri("/api/v1/users/me/email-change")
        .cookie(Cookie::new("access", access.clone()))
        .set_json(serde_json::json!({ "new_email": "audit-ec-target-final@example.com" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 429, "sixth attempt within the window must be 429");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "rate_limited");
}

// ---------------------------------------------------------------------------
// MED: person free-text field length caps (cf0c9b2)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn person_fields_reject_overlong_strings() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let (access, _r) = sign_in(&stack, &app, "audit-pl@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Lengths").await;

    // given_name cap is 200 chars.
    let long_name = "x".repeat(201);
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({ "given_name": long_name }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422, "201-char given_name must 422");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.string_too_long");
    assert_eq!(body["fields"][0]["path"], "/given_name");

    // notes cap is 2000 chars (highest of the person fields).
    let long_notes = "x".repeat(2001);
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({ "given_name": "OK", "notes": long_notes }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422, "2001-char notes must 422");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["path"], "/notes");

    // gender cap is 100 chars (short bucket).
    let long_gender = "x".repeat(101);
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({ "given_name": "OK", "gender": long_gender }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422, "101-char gender must 422");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["path"], "/gender");
}

// ---------------------------------------------------------------------------
// MED: family name length cap + CR/LF rejection (cf0c9b2)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn family_name_rejects_newlines_and_overlong_input() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let (access, _r) = sign_in(&stack, &app, "audit-fn@example.com").await;

    // Newline in family name — header-injection hardening on the email
    // Subject path (lettre already escapes, but the validator is the
    // first line of defence).
    let req = test::TestRequest::post()
        .uri("/api/v1/families")
        .cookie(Cookie::new("access", access.clone()))
        .set_json(serde_json::json!({ "name": "Schmidt\nInjected: x" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422, "newline in name must 422");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.value_invalid");

    // Length cap (200 chars).
    let long = "x".repeat(201);
    let req = test::TestRequest::post()
        .uri("/api/v1/families")
        .cookie(Cookie::new("access", access.clone()))
        .set_json(serde_json::json!({ "name": long }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422, "overlong name must 422");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.string_too_long");
}
