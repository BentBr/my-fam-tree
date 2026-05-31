//! End-to-end auth + family flow against ephemeral Postgres + Redis containers
//! plus a `FakeEmailSender`.
//!
//! Verifies the full chain:
//!
//! 1. `POST /auth/magic-link` creates the user, persists the hashed token,
//!    and queues an email.
//! 2. `POST /auth/consume` exchanges the magic-link token for an `access`
//!    cookie + `refresh` cookie + JSON claims payload.
//! 3. `GET /auth/me` echoes the verified claims.
//! 4. `POST /families` mints a fresh access cookie that reflects the new
//!    `Owner` membership.
//! 5. `GET /auth/me` with the new cookie sees the family.
//! 6. `POST /auth/refresh` rotates both cookies.
//!
//! The container handles are owned by the test (NOT `Box::leak`-ed) so their
//! `Drop` impls trigger the testcontainers reaper. Without that we'd accumulate
//! orphan Postgres + Redis containers on every run.

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
use common::{ephemeral_stack, extract_token_from_link};
use my_fam_tree_api::build_app;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn magic_link_then_consume_then_me_then_create_family_then_refresh() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    // 1. Request a magic link.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/magic-link")
        .set_json(serde_json::json!({ "email": "anna@example.com" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200, "magic-link request should succeed");

    // 2. Grab the email and extract the opaque token.
    let captured = stack.fake_email.drain();
    assert_eq!(captured.len(), 1, "exactly one magic-link email expected");
    let token = extract_token_from_link(&captured[0].text_body);
    assert!(!token.is_empty(), "extracted token must be non-empty");

    // 3. Consume — sets both cookies and returns the claims payload.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/consume")
        .set_json(serde_json::json!({ "token": token }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200, "consume should succeed");
    let access = res.response().cookies().find(|c| c.name() == "access").expect("access cookie");
    let access_value = access.value().to_string();
    let refresh = res.response().cookies().find(|c| c.name() == "refresh").expect("refresh cookie");
    let refresh_value = refresh.value().to_string();

    // 4. /auth/me echoes the freshly minted session.
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .cookie(Cookie::new("access", access_value.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["email"], "anna@example.com");
    assert_eq!(body["data"]["families"].as_array().unwrap().len(), 0);

    // 5. Create a family — handler should reissue the access cookie so the
    //    new Owner membership is immediately visible.
    let req = test::TestRequest::post()
        .uri("/api/v1/families")
        .cookie(Cookie::new("access", access_value))
        .set_json(serde_json::json!({ "name": "Müller" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let new_access =
        res.response().cookies().find(|c| c.name() == "access").expect("new access cookie");
    let new_access_value = new_access.value().to_string();
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["family"]["name"], "Müller");
    assert_eq!(body["data"]["claims"]["families"].as_array().unwrap().len(), 1);

    // 6. /auth/me with the rotated cookie reflects the membership.
    let req = test::TestRequest::get()
        .uri("/api/v1/auth/me")
        .cookie(Cookie::new("access", new_access_value))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["families"][0]["role"], "owner");

    // 7. Refresh round-trip: the refresh cookie path is /api/v1/auth/refresh,
    //    so the test cookie passes through fine.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .cookie(Cookie::new("refresh", refresh_value))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    assert!(res.response().cookies().any(|c| c.name() == "access"));
    assert!(res.response().cookies().any(|c| c.name() == "refresh"));
}

/// Regression: `POST /auth/logout` MUST be reachable without any cookies
/// and MUST still emit clear-cookie headers. The FE relies on this to
/// drop stale `HttpOnly` cookies AFTER a session has already collapsed
/// server-side (e.g. refresh failed) — at that point the access cookie
/// is gone, so an auth-gated logout would 401 and the browser would
/// keep the cookies until their natural TTL.
///
/// The previous shape mounted logout INSIDE the required-auth scope; a
/// no-cookie POST returned 401. After moving the registration up
/// alongside the other public auth endpoints, it now returns 200 + the
/// `Set-Cookie max-age=0` pair for both cookies. Body is the fixed
/// `LogoutRes`, no session info leaked.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn logout_is_idempotent_and_reachable_without_session() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let req = test::TestRequest::post().uri("/api/v1/auth/logout").to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200, "logout must succeed even without a session");

    // The response carries Set-Cookie clearing headers for BOTH cookies
    // (so the browser drops whatever it still has) with max-age=0.
    let cookies: Vec<_> = res.response().cookies().collect();
    let access = cookies.iter().find(|c| c.name() == "access").expect("access clear cookie");
    let refresh = cookies.iter().find(|c| c.name() == "refresh").expect("refresh clear cookie");
    assert_eq!(access.value(), "", "cleared access cookie is empty");
    assert_eq!(refresh.value(), "", "cleared refresh cookie is empty");
    assert_eq!(
        access.max_age().expect("access cookie has Max-Age"),
        actix_web::cookie::time::Duration::seconds(0),
        "access cookie must have Max-Age=0"
    );
    assert_eq!(
        refresh.max_age().expect("refresh cookie has Max-Age"),
        actix_web::cookie::time::Duration::seconds(0),
        "refresh cookie must have Max-Age=0"
    );

    // Body shape: fixed status string, no session info.
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["status"], "logged out");
}
