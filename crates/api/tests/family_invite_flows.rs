//! Adversarial / happy-path coverage for the family + invite handlers.
//!
//! Like `auth_errors.rs`, each test spins its own Postgres + Redis pair via
//! `ephemeral_stack()`. These tests exercise the validation, membership, and
//! invite-token paths that ride on top of the auth chain covered elsewhere.

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
use my_family_api::build_app;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_family_rejects_empty_name() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _refresh) = sign_in(&stack, &app, "empty-name@example.com").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/families")
        .cookie(Cookie::new("access", access))
        .set_json(serde_json::json!({ "name": "   " }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.value_required");
    assert_eq!(body["fields"][0]["path"], "/name");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invite_rejects_owner_role_and_invalid_email() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _refresh) = sign_in(&stack, &app, "owner@example.com").await;

    // Create a family so we have a valid family id and Owner membership.
    let req = test::TestRequest::post()
        .uri("/api/v1/families")
        .cookie(Cookie::new("access", access))
        .set_json(serde_json::json!({ "name": "Owners" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let owner_access = res
        .response()
        .cookies()
        .find(|c| c.name() == "access")
        .expect("rotated access")
        .value()
        .to_string();
    let body: serde_json::Value = test::read_body_json(res).await;
    let family_id = body["data"]["family"]["id"].as_str().expect("family id").to_string();

    // Inviting as owner is disallowed by the validation rule.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id}/invites"))
        .cookie(Cookie::new("access", owner_access.clone()))
        .set_json(serde_json::json!({ "email": "guest@example.com", "role": "owner" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.role_invalid");
    assert_eq!(body["fields"][0]["path"], "/role");

    // Inviting with a malformed email also fails validation.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id}/invites"))
        .cookie(Cookie::new("access", owner_access))
        .set_json(serde_json::json!({ "email": "not-an-email", "role": "user" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.email_invalid");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invite_accept_happy_path_and_email_mismatch() {
    // End-to-end invite flow. The accept handler is mark-and-fetch atomic —
    // the invite row is flipped to `accepted_at=now()` BEFORE the
    // signed-in-email check runs, so a wrong-email attempt also burns the
    // invite. We use two separate invites here to cover both arms cleanly:
    //   - invite #1 for user-mismatch@... is accepted by user-c@... -> 422
    //     `validation.invite_email_mismatch`.
    //   - invite #2 for user-b@...        is accepted by user-b@...  -> 200
    //     with the rotated access cookie listing the family as Admin.
    //   - a third accept with a bogus token returns 401 (NotFoundOrAccepted).
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let (access_a, _refresh_a) = sign_in(&stack, &app, "user-a@example.com").await;
    // Drain the magic-link email so subsequent `last()` calls land on invites.
    stack.fake_email.drain();

    let req = test::TestRequest::post()
        .uri("/api/v1/families")
        .cookie(Cookie::new("access", access_a))
        .set_json(serde_json::json!({ "name": "Alpha" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let rotated_access_a = res
        .response()
        .cookies()
        .find(|c| c.name() == "access")
        .expect("rotated access")
        .value()
        .to_string();
    let body: serde_json::Value = test::read_body_json(res).await;
    let family_id = body["data"]["family"]["id"].as_str().expect("family id").to_string();

    // Invite #1: addressed to user-mismatch@..., used to drive the email
    // mismatch arm without burning the user-b@... invite.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id}/invites"))
        .cookie(Cookie::new("access", rotated_access_a.clone()))
        .set_json(serde_json::json!({ "email": "user-mismatch@example.com", "role": "admin" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let token_mismatch = {
        let captured = stack.fake_email.drain();
        let mail = captured.last().expect("invite email captured");
        assert_eq!(mail.to_addr, "user-mismatch@example.com");
        extract_token_from_link(&mail.text_body)
    };

    // Invite #2: the one user B will accept.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id}/invites"))
        .cookie(Cookie::new("access", rotated_access_a))
        .set_json(serde_json::json!({ "email": "user-b@example.com", "role": "admin" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let token_b = {
        let captured = stack.fake_email.drain();
        let mail = captured.last().expect("invite email captured");
        assert_eq!(mail.to_addr, "user-b@example.com");
        extract_token_from_link(&mail.text_body)
    };

    // User C tries to accept invite #1 — different email, validation fires.
    let (access_c, _refresh_c) = sign_in(&stack, &app, "user-c@example.com").await;
    let req = test::TestRequest::post()
        .uri("/api/v1/invites/accept")
        .cookie(Cookie::new("access", access_c))
        .set_json(serde_json::json!({ "token": token_mismatch }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.invite_email_mismatch");

    // User B signs in and accepts invite #2 — happy path.
    let (access_b, _refresh_b) = sign_in(&stack, &app, "user-b@example.com").await;
    let req = test::TestRequest::post()
        .uri("/api/v1/invites/accept")
        .cookie(Cookie::new("access", access_b))
        .set_json(serde_json::json!({ "token": token_b }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let new_access_value = res
        .response()
        .cookies()
        .find(|c| c.name() == "access")
        .expect("access")
        .value()
        .to_string();
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["family"]["role"], "admin");

    // A subsequent accept with a bogus token returns 401 — the hash doesn't
    // match any row, so the repo's `NotFoundOrAccepted` arm maps to
    // `MagicLinkInvalid` (401, `auth_magic_link_invalid`).
    let req = test::TestRequest::post()
        .uri("/api/v1/invites/accept")
        .cookie(Cookie::new("access", new_access_value))
        .set_json(serde_json::json!({ "token": "bad-token" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401, "missing/used token must be MagicLinkInvalid");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_magic_link_invalid");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn family_operations_require_membership_and_role() {
    // Two users, two families. A owns Alpha, B owns Beta.
    //   - A renaming Beta -> 403 (not a member).
    //   - A inviting to Beta -> 403 (not a member).
    //   - A deleting Beta -> 403 (not a member).
    //   - A renaming Alpha with empty name -> 422.
    //   - GET /families/me with no session -> 401.
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let (access_a, _r_a) = sign_in(&stack, &app, "user-a2@example.com").await;
    let req = test::TestRequest::post()
        .uri("/api/v1/families")
        .cookie(Cookie::new("access", access_a))
        .set_json(serde_json::json!({ "name": "Alpha" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    let access_a_rot = res
        .response()
        .cookies()
        .find(|c| c.name() == "access")
        .expect("access a")
        .value()
        .to_string();
    let body: serde_json::Value = test::read_body_json(res).await;
    let family_alpha = body["data"]["family"]["id"].as_str().expect("alpha id").to_string();

    let (access_b, _r_b) = sign_in(&stack, &app, "user-b2@example.com").await;
    let req = test::TestRequest::post()
        .uri("/api/v1/families")
        .cookie(Cookie::new("access", access_b))
        .set_json(serde_json::json!({ "name": "Beta" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    let family_beta = body["data"]["family"]["id"].as_str().expect("beta id").to_string();

    // A tries to rename Beta — 403 NotFamilyMember.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/families/{family_beta}"))
        .cookie(Cookie::new("access", access_a_rot.clone()))
        .set_json(serde_json::json!({ "name": "Stolen" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "family_not_member");

    // A tries to invite into Beta — same 403.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_beta}/invites"))
        .cookie(Cookie::new("access", access_a_rot.clone()))
        .set_json(serde_json::json!({ "email": "x@example.com", "role": "user" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);

    // A tries to delete Beta — same 403.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/families/{family_beta}"))
        .cookie(Cookie::new("access", access_a_rot.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);

    // A renames Alpha with an empty name — 422.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/families/{family_alpha}"))
        .cookie(Cookie::new("access", access_a_rot.clone()))
        .set_json(serde_json::json!({ "name": "   " }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.value_required");

    // A actually renames Alpha — happy patch.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/families/{family_alpha}"))
        .cookie(Cookie::new("access", access_a_rot.clone()))
        .set_json(serde_json::json!({ "name": "Alpha Renamed" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["name"], "Alpha Renamed");

    // GET /families/me without a session — 401 from AuthMiddleware.
    let req = test::TestRequest::get().uri("/api/v1/families/me").to_request();
    let res = try_call(&app, req).await;
    assert_eq!(res.status(), 401);

    // GET /families/me with A's session — 1 membership.
    let req = test::TestRequest::get()
        .uri("/api/v1/families/me")
        .cookie(Cookie::new("access", access_a_rot.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["families"].as_array().unwrap().len(), 1);

    // A deletes Alpha — happy path covers the DELETE handler success arm.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/families/{family_alpha}"))
        .cookie(Cookie::new("access", access_a_rot))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn protected_endpoints_require_authentication() {
    // Every endpoint mounted under the `AuthMiddleware::required()` scope must
    // reject anonymous (no `access` cookie) callers with a 401 + RFC 7807
    // envelope carrying `code = "auth_unauthenticated"`. `auth_errors.rs`
    // already pins this for `/auth/logout`; this consolidated check guards the
    // family + invite surface so a future refactor that drops the middleware
    // wrap on any one route is caught here.
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    // The `00000000-...` UUID is structurally valid (so path extraction passes)
    // but obviously doesn't reference a real row; we never reach the handler
    // since the middleware rejects before extraction runs anyway.
    let endpoints: Vec<(&'static str, &'static str, serde_json::Value)> = vec![
        ("GET", "/api/v1/auth/me", serde_json::Value::Null),
        ("POST", "/api/v1/auth/logout", serde_json::Value::Null),
        ("GET", "/api/v1/families/me", serde_json::Value::Null),
        ("POST", "/api/v1/families", serde_json::json!({ "name": "x" })),
        (
            "PATCH",
            "/api/v1/families/00000000-0000-0000-0000-000000000000",
            serde_json::json!({ "name": "y" }),
        ),
        (
            "DELETE",
            "/api/v1/families/00000000-0000-0000-0000-000000000000",
            serde_json::Value::Null,
        ),
        (
            "POST",
            "/api/v1/families/00000000-0000-0000-0000-000000000000/invites",
            serde_json::json!({ "email": "a@b.co", "role": "user" }),
        ),
        ("POST", "/api/v1/invites/accept", serde_json::json!({ "token": "x" })),
    ];

    for (method, path, body) in endpoints {
        let mut req = match method {
            "GET" => test::TestRequest::get().uri(path),
            "POST" => test::TestRequest::post().uri(path),
            "PATCH" => test::TestRequest::patch().uri(path),
            "DELETE" => test::TestRequest::delete().uri(path),
            other => unreachable!("unsupported method {other}"),
        };
        if !body.is_null() {
            req = req.set_json(&body);
        }
        let res = try_call(&app, req.to_request()).await;
        assert_eq!(res.status(), 401, "expected 401 for {method} {path}, got {}", res.status());
        let body_bytes = test::read_body(res).await;
        let json: serde_json::Value =
            serde_json::from_slice(&body_bytes).expect("response body is json");
        assert_eq!(json["code"], "auth_unauthenticated", "wrong code for {method} {path}");
    }
}
