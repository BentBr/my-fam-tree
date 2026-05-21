//! Integration coverage for `/users/me` and the email-change flow.
//!
//! Each test spins its own Postgres + Redis pair via `ephemeral_stack()`.
//! These tests exercise the `users` route surface added in Phase 1c — fresh
//! profile reads, partial updates, and the two-step old-mail-confirms
//! email-change handshake (request → magic link → confirm).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::shadow_unrelated,
    clippy::shadow_reuse,
    clippy::shadow_same
)]

mod common;

use actix_web::cookie::Cookie;
use actix_web::test;
use common::{ephemeral_stack, extract_token_from_link, sign_in};
use my_family_api::build_app;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_profile_get_and_update() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _refresh) = sign_in(&stack, &app, "profile-user@example.com").await;

    // GET /users/me returns the freshly-created user.
    let req = test::TestRequest::get()
        .uri("/api/v1/users/me")
        .cookie(Cookie::new("access", access.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["email"], "profile-user@example.com");
    assert_eq!(body["data"]["display_name"], "");
    assert_eq!(body["data"]["locale"], "en");

    // PATCH /users/me updates both fields.
    let req = test::TestRequest::patch()
        .uri("/api/v1/users/me")
        .cookie(Cookie::new("access", access.clone()))
        .set_json(serde_json::json!({ "display_name": "Ada Lovelace", "locale": "de" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["display_name"], "Ada Lovelace");
    assert_eq!(body["data"]["locale"], "de");

    // Second GET confirms persistence.
    let req = test::TestRequest::get()
        .uri("/api/v1/users/me")
        .cookie(Cookie::new("access", access.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["display_name"], "Ada Lovelace");
    assert_eq!(body["data"]["locale"], "de");

    // Empty body → 422 value_required on the root.
    let req = test::TestRequest::patch()
        .uri("/api/v1/users/me")
        .cookie(Cookie::new("access", access.clone()))
        .set_json(serde_json::json!({}))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.value_required");

    // Blank display_name → 422 value_required on /display_name.
    let req = test::TestRequest::patch()
        .uri("/api/v1/users/me")
        .cookie(Cookie::new("access", access.clone()))
        .set_json(serde_json::json!({ "display_name": "   " }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["path"], "/display_name");
    assert_eq!(body["fields"][0]["code"], "validation.value_required");

    // Unknown locale → 422 locale_invalid.
    let req = test::TestRequest::patch()
        .uri("/api/v1/users/me")
        .cookie(Cookie::new("access", access.clone()))
        .set_json(serde_json::json!({ "locale": "fr" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.locale_invalid");

    // Overlong display_name → 422 string_too_long with max=100.
    let too_long = "x".repeat(101);
    let req = test::TestRequest::patch()
        .uri("/api/v1/users/me")
        .cookie(Cookie::new("access", access))
        .set_json(serde_json::json!({ "display_name": too_long }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.string_too_long");
    assert_eq!(body["fields"][0]["params"]["max"], 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_email_change_old_email_confirms() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _refresh) = sign_in(&stack, &app, "old-mail@example.com").await;
    stack.fake_email.drain();

    // Request the change. The link is sent to the OLD address.
    let req = test::TestRequest::post()
        .uri("/api/v1/users/me/email-change")
        .cookie(Cookie::new("access", access.clone()))
        .set_json(serde_json::json!({ "new_email": "new-mail@example.com" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["status"], "pending");

    // The confirmation email goes to the OLD address but mentions the NEW.
    let token = {
        let captured = stack.fake_email.drain();
        let mail = captured.last().expect("email-change email captured");
        assert_eq!(mail.to_addr, "old-mail@example.com");
        assert!(mail.text_body.contains("new-mail@example.com"));
        extract_token_from_link(&mail.text_body)
    };

    // Confirm using the same session — the handler cross-checks user_id.
    let req = test::TestRequest::post()
        .uri("/api/v1/users/me/email-change/confirm")
        .cookie(Cookie::new("access", access.clone()))
        .set_json(serde_json::json!({ "token": token }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["email"], "new-mail@example.com");
    assert!(body["data"]["email_verified_at"].is_null());

    // A second confirm with the same token is rejected (consumed).
    let req = test::TestRequest::post()
        .uri("/api/v1/users/me/email-change/confirm")
        .cookie(Cookie::new("access", access))
        .set_json(serde_json::json!({ "token": token }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_magic_link_invalid");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_email_change_rejects_invalid_email() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _refresh) = sign_in(&stack, &app, "bad-input@example.com").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/users/me/email-change")
        .cookie(Cookie::new("access", access))
        .set_json(serde_json::json!({ "new_email": "not-an-email" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.email_invalid");
    assert_eq!(body["fields"][0]["path"], "/new_email");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_email_change_rejects_same_email() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _refresh) = sign_in(&stack, &app, "same@example.com").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/users/me/email-change")
        .cookie(Cookie::new("access", access))
        .set_json(serde_json::json!({ "new_email": "same@example.com" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.email_same_as_current");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_email_change_rejects_taken_email() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    // Pre-create the "taken" account.
    let _ = sign_in(&stack, &app, "taken@example.com").await;
    stack.fake_email.drain();

    let (access, _refresh) = sign_in(&stack, &app, "hopeful@example.com").await;
    let req = test::TestRequest::post()
        .uri("/api/v1/users/me/email-change")
        .cookie(Cookie::new("access", access))
        .set_json(serde_json::json!({ "new_email": "taken@example.com" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 409);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "email_taken");
}
