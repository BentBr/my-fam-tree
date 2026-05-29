//! Adversarial / error-path coverage for the auth handlers.
//!
//! These tests share `ephemeral_stack()` but each spins its own Postgres +
//! Redis pair so they can run in parallel without crosstalk. They exercise
//! the error arms of the magic-link, consume, refresh, and logout handlers —
//! the happy path is covered by `auth_flow.rs`.

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
use common::{ephemeral_stack, extract_token_from_link, sign_in, try_call};
use my_fam_tree_api::build_app;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn magic_link_rejects_invalid_email() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/magic-link")
        .set_json(serde_json::json!({ "email": "nope" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422, "syntactically broken email must be rejected");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "validation_failed");
    assert_eq!(body["fields"][0]["code"], "validation.email_invalid");
    assert_eq!(body["fields"][0]["path"], "/email");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn consume_rejects_empty_unknown_and_replayed_tokens() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    // Empty token — short-circuits before any DB hit.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/consume")
        .set_json(serde_json::json!({ "token": "" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_magic_link_invalid");

    // Unknown opaque token — the hash exists, just not in the table.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/consume")
        .set_json(serde_json::json!({ "token": "deadbeefdeadbeefdeadbeefdeadbeef" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_magic_link_invalid");

    // Replay: consume a real token twice. Issue a magic link, consume once,
    // then consume the same token again — the second call must 401.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/magic-link")
        .set_json(serde_json::json!({ "email": "replay@example.com" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let captured = stack.fake_email.drain();
    let token = extract_token_from_link(&captured.last().expect("email").text_body);

    let first = test::TestRequest::post()
        .uri("/api/v1/auth/consume")
        .set_json(serde_json::json!({ "token": token.clone() }))
        .to_request();
    let res = test::call_service(&app, first).await;
    assert_eq!(res.status(), 200, "first consume should succeed");

    let second = test::TestRequest::post()
        .uri("/api/v1/auth/consume")
        .set_json(serde_json::json!({ "token": token }))
        .to_request();
    let res = test::call_service(&app, second).await;
    assert_eq!(res.status(), 401, "replayed token must be rejected");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_magic_link_invalid");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refresh_rejects_missing_and_bogus_cookies() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    // No cookie at all.
    let req = test::TestRequest::post().uri("/api/v1/auth/refresh").to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_refresh_invalid");

    // Garbage cookie — hashes to something the DB doesn't know about.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .cookie(Cookie::new("refresh", "garbage-not-a-real-token"))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_refresh_invalid");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logout_without_session_returns_unauthenticated() {
    // /auth/logout sits behind AuthMiddleware::required, so calling it
    // without an access cookie is a 401 — not the idempotent 200 you'd get
    // from a session-less logout endpoint. This pins that behaviour and
    // exercises the middleware's missing-cookie arm.
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let req = test::TestRequest::post().uri("/api/v1/auth/logout").to_request();
    let res = try_call(&app, req).await;
    assert_eq!(res.status(), 401);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_unauthenticated");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logout_with_session_clears_cookies_and_revokes_refresh() {
    // Sign in, then logout with both cookies. The handler should:
    //  - return 200,
    //  - emit cleared `access` + `refresh` cookies (expired Max-Age),
    //  - revoke the underlying refresh row (verified indirectly: a refresh
    //    call with the same cookie afterwards must 401).
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, refresh) = sign_in(&stack, &app, "logout-me@example.com").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/auth/logout")
        .cookie(Cookie::new("access", access.clone()))
        .cookie(Cookie::new("refresh", refresh.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let cleared_access = res.response().cookies().find(|c| c.name() == "access").expect("access");
    let cleared_refresh =
        res.response().cookies().find(|c| c.name() == "refresh").expect("refresh");
    assert!(cleared_access.value().is_empty(), "access cookie should be cleared");
    assert!(cleared_refresh.value().is_empty(), "refresh cookie should be cleared");

    // The refresh row must be revoked — a subsequent /auth/refresh 401s.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .cookie(Cookie::new("refresh", refresh))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401);
}
