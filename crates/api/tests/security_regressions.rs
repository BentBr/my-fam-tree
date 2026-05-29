//! Regression coverage for the security audit fixes shipped in
//! `cf0c9b2` (1 High + 5 Medium), `964c299` (4 Low),
//! `a420ccd` (Info: `deny_unknown_fields`), and `d984474` (Info: token IP caps).
//!
//! One test per finding — small and surgical, each pinning the exact
//! attacker shape the audit described so a future regression flips the
//! red bit in the same place.
//!
//! Tests for the token-endpoint per-IP caps (consume / refresh / accept /
//! owner-transfer-confirm — `d984474`) are intentionally omitted: those
//! cap at 120/hour each, so a regression test would need 120+ successful
//! sign-ins per test which dominates runtime without changing the
//! contract being asserted. The cap on `/users/me/email-change` is 5/hour
//! and IS tested below.

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
use my_family_api::build_app;

/// Create a person and return its id. Mirrors the helper in
/// `person_photo_flow.rs` so each test file stays self-contained.
#[allow(clippy::future_not_send)]
async fn create_person<S, B>(app: &S, access: &str, family_id: &str, given_name: &str) -> String
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.to_string()))
        .insert_header(("X-Family-Id", family_id.to_string()))
        .set_json(serde_json::json!({ "given_name": given_name }))
        .to_request();
    let res = test::call_service(app, req).await;
    assert_eq!(res.status(), 200, "create person `{given_name}` should succeed");
    let body: serde_json::Value = test::read_body_json(res).await;
    body["data"]["id"].as_str().expect("person id").to_string()
}

// ---------------------------------------------------------------------------
// HIGH: stale-role privilege window (require_db_role)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn demoted_admin_loses_privilege_immediately_not_at_token_ttl() {
    // Original shape: admin's access cookie carries `role=admin` baked in
    // at issue time. After the owner demotes them to `user` in the DB,
    // the cookie still claims admin until access TTL expires (15 min).
    // The DB-level role check now fetches the live membership row so
    // the next privileged write rejects with insufficient_role.
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let (owner_access, _r) = sign_in(&stack, &app, "stale-owner@example.com").await;
    let (_owner_access, family_id) = create_family(&app, &owner_access, "StaleRole").await;
    let (_admin_access, _r) = sign_in(&stack, &app, "stale-admin@example.com").await;

    // Insert admin's membership directly through the repo: cheaper than
    // running the full invite flow and gives us a clean role state.
    let admin_user_id = stack
        .state
        .users
        .find_by_email("stale-admin@example.com")
        .await
        .expect("find admin")
        .expect("admin exists")
        .id;
    let fid = my_family_domain::FamilyId::from_uuid(
        uuid::Uuid::parse_str(&family_id).expect("family uuid"),
    );
    stack
        .state
        .memberships
        .insert(fid, admin_user_id, my_family_domain::Role::Admin)
        .await
        .expect("admin membership");

    // Sign the admin back in so the access cookie reflects the admin role
    // (otherwise it would still say "user" from before the row was
    // inserted — the JWT only includes families the user already
    // belonged to at issuance).
    let (admin_access, _r) = sign_in(&stack, &app, "stale-admin@example.com").await;

    // Sanity: admin CAN list members while still admin in DB.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/families/{family_id}/members"))
        .cookie(Cookie::new("access", admin_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200, "admin should be able to list members");

    // Owner demotes admin to plain user via the DB (mirrors what
    // members::set_member_role would do).
    stack
        .state
        .memberships
        .set_role(fid, admin_user_id, my_family_domain::Role::User)
        .await
        .expect("demote admin");

    // The admin's access cookie still claims admin (JWT is unchanged).
    // The next privileged read must reject with 403 because the DB-level
    // check sees the real role now.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/families/{family_id}/members"))
        .cookie(Cookie::new("access", admin_access))
        .insert_header(("X-Family-Id", family_id))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403, "demoted admin must lose access immediately");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "family_insufficient_role");
}

// ---------------------------------------------------------------------------
// HIGH: parent_links cross-family IDOR (cf0c9b2)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parent_links_create_rejects_cross_family_child_or_parent() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let (access_a, _r) = sign_in(&stack, &app, "audit-pl-a@example.com").await;
    let (access_a, family_a) = create_family(&app, &access_a, "F-A").await;
    let child_a = create_person(&app, &access_a, &family_a, "Child").await;
    let parent_a = create_person(&app, &access_a, &family_a, "Parent").await;

    let (access_b, _r) = sign_in(&stack, &app, "audit-pl-b@example.com").await;
    let (access_b, family_b) = create_family(&app, &access_b, "F-B").await;

    // Attacker in family B tries to plant a parent_link between two F-A persons.
    let req = test::TestRequest::post()
        .uri("/api/v1/parent-links")
        .cookie(Cookie::new("access", access_b.clone()))
        .insert_header(("X-Family-Id", family_b))
        .set_json(serde_json::json!({
            "child_id": child_a,
            "parent_id": parent_a,
            "kind": "biological",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 404, "cross-family parent_link must 404");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "person_not_found");
}

// ---------------------------------------------------------------------------
// MED: partnerships cross-family IDOR (cf0c9b2)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn partnerships_create_rejects_cross_family_partner_ids() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let (access_a, _r) = sign_in(&stack, &app, "audit-pa-a@example.com").await;
    let (access_a, family_a) = create_family(&app, &access_a, "Audit-A").await;
    let p_a = create_person(&app, &access_a, &family_a, "PartnerA").await;
    let p_b = create_person(&app, &access_a, &family_a, "PartnerB").await;

    let (access_b, _r) = sign_in(&stack, &app, "audit-pa-b@example.com").await;
    let (access_b, family_b) = create_family(&app, &access_b, "Audit-B").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/partnerships")
        .cookie(Cookie::new("access", access_b.clone()))
        .insert_header(("X-Family-Id", family_b))
        .set_json(serde_json::json!({
            "partner_a_id": p_a,
            "partner_b_id": p_b,
            "kind": "marriage",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 404, "cross-family partnership must 404");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "person_not_found");
}

// ---------------------------------------------------------------------------
// MED: family_invites cross-family person_id IDOR (cf0c9b2)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn family_invite_rejects_cross_family_person_id() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let (access_a, _r) = sign_in(&stack, &app, "audit-fi-a@example.com").await;
    let (access_a, family_a) = create_family(&app, &access_a, "Inv-A").await;
    let person_a = create_person(&app, &access_a, &family_a, "TargetPerson").await;

    let (access_b, _r) = sign_in(&stack, &app, "audit-fi-b@example.com").await;
    let (access_b, family_b) = create_family(&app, &access_b, "Inv-B").await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_b}/invites"))
        .cookie(Cookie::new("access", access_b.clone()))
        .set_json(serde_json::json!({
            "email": "outsider@example.com",
            "role": "user",
            "person_id": person_a,
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 404, "invite referencing a foreign person must 404");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "person_not_found");
}

// ---------------------------------------------------------------------------
// LOW: EmailTaken response no longer echoes the submitted address (964c299)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn email_taken_response_does_not_echo_the_email() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    // Two existing users — A is the caller, B owns the target address.
    let (access_a, _r) = sign_in(&stack, &app, "audit-et-caller@example.com").await;
    let _ = sign_in(&stack, &app, "audit-et-owner@example.com").await;

    // Caller tries to swap their email to B's address.
    let req = test::TestRequest::post()
        .uri("/api/v1/users/me/email-change")
        .cookie(Cookie::new("access", access_a.clone()))
        .set_json(serde_json::json!({ "new_email": "audit-et-owner@example.com" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 409, "duplicate email returns 409");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "email_taken");

    // Critical: the response body must not name the address — that was the
    // existence-probe vector. The unit-variant variant emits a generic
    // "email already in use" detail.
    let payload = serde_json::to_string(&body).expect("serialise back");
    assert!(
        !payload.contains("audit-et-owner@example.com"),
        "EmailTaken body must not echo the email; got: {payload}",
    );
}

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
