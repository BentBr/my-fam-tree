//! Integration tests for Phase E — owner transfer (double-verification).
//!
//! Each test spins up an `ephemeral_stack` and seeds an owner + admin
//! pair on a fresh family. The owner POSTs `/transfer-owner` which
//! dispatches two emails — one to the current owner, one to the target
//! admin. Each side's `confirm` route advances the state machine; when
//! BOTH sides have clicked their link, the role swap commits atomically.

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
use my_family_api::{AppState, build_app};
use my_family_domain::{AuditFilter, FamilyId, Role, UserId};
use uuid::Uuid;

async fn provision_user<S, B>(stack: &common::TestStack, app: &S, email: &str) -> UserId
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let _ = sign_in(stack, app, email).await;
    stack.state.users.find_by_email(email).await.expect("user lookup").expect("user exists").id
}

async fn ensure_membership(state: &AppState, family_id: FamilyId, user_id: UserId, role: Role) {
    if state.memberships.find(family_id, user_id).await.expect("find").is_some() {
        state.memberships.set_role(family_id, user_id, role).await.expect("set_role");
    } else {
        state.memberships.insert(family_id, user_id, role).await.expect("insert");
    }
}

struct OwnerAdminFamily {
    family_id: FamilyId,
    owner_email: String,
    admin_email: String,
    owner_id: UserId,
    admin_id: UserId,
}

async fn seed_owner_admin_family<S, B>(
    stack: &common::TestStack,
    app: &S,
    stamp: u128,
) -> OwnerAdminFamily
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let owner_email = format!("xfer-owner-{stamp}@example.com");
    let admin_email = format!("xfer-admin-{stamp}@example.com");

    let (owner_access, _r) = sign_in(stack, app, &owner_email).await;
    let (_owner_access, family_id_str) =
        common::create_family(app, &owner_access, &format!("XferFam-{stamp}")).await;
    let family_uuid: Uuid = family_id_str.parse().expect("uuid");
    let family_id = FamilyId::from_uuid(family_uuid);

    let owner_id = stack
        .state
        .users
        .find_by_email(&owner_email)
        .await
        .expect("user lookup")
        .expect("owner exists")
        .id;
    let admin_id = provision_user(stack, app, &admin_email).await;
    ensure_membership(&stack.state, family_id, admin_id, Role::Admin).await;

    OwnerAdminFamily { family_id, owner_email, admin_email, owner_id, admin_id }
}

async fn fresh_access<S, B>(stack: &common::TestStack, app: &S, email: &str) -> String
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let (access, _r) = sign_in(stack, app, email).await;
    access
}

/// Pull `token` from a `…?token=XYZ` URL out of an email body. The token is
/// url-safe base64 (no padding), so any whitespace / quote terminates it.
fn extract_transfer_token(body: &str) -> String {
    let after = body.split("token=").nth(1).expect("token= present");
    after.split(|c: char| c.is_whitespace() || c == '"').next().expect("token chars").to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owner_begins_transfer_to_admin_emits_two_emails() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let fam = seed_owner_admin_family(&stack, &app, stamp).await;

    // Drain whatever sign-in mails the stack captured so we only see the
    // two transfer mails.
    let _ = stack.fake_email.drain();

    let access = fresh_access(&stack, &app, &fam.owner_email).await;
    let _ = stack.fake_email.drain();
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{}/transfer-owner", fam.family_id.into_uuid()))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", fam.family_id.into_uuid().to_string()))
        .set_json(serde_json::json!({ "to_user_id": fam.admin_id.into_uuid() }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);

    let outbox = stack.fake_email.drain();
    assert_eq!(outbox.len(), 2, "expected two transfer emails, got {outbox:?}");
    let subjects: Vec<&str> = outbox.iter().map(|e| e.subject.as_str()).collect();
    assert!(
        subjects.iter().any(|s| s.contains("Confirm ownership")),
        "owner-side subject missing in {subjects:?}",
    );
    assert!(
        subjects.iter().any(|s| s.contains("offered ownership")),
        "admin-side subject missing in {subjects:?}",
    );

    // Audit `begin / owner_transfer` row written.
    let (rows, _) = stack
        .state
        .audit
        .list_filtered(AuditFilter {
            family_id: fam.family_id,
            from: None,
            to: None,
            action: Some("begin".into()),
            entity_kind: Some("owner_transfer".into()),
            actor_user_id: None,
            page: 1,
            page_size: 50,
        })
        .await
        .expect("audit list");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].metadata["to_user_id"], fam.admin_id.into_uuid().to_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn both_sides_confirm_completes_transfer_and_swaps_roles() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let fam = seed_owner_admin_family(&stack, &app, stamp).await;

    let _ = stack.fake_email.drain();
    let owner_access = fresh_access(&stack, &app, &fam.owner_email).await;
    let _ = stack.fake_email.drain();
    let _ = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/v1/families/{}/transfer-owner", fam.family_id.into_uuid()))
            .cookie(Cookie::new("access", owner_access.clone()))
            .insert_header(("X-Family-Id", fam.family_id.into_uuid().to_string()))
            .set_json(serde_json::json!({ "to_user_id": fam.admin_id.into_uuid() }))
            .to_request(),
    )
    .await;

    let outbox = stack.fake_email.drain();
    assert_eq!(outbox.len(), 2);
    // First mail goes to the current owner (the "confirm" subject), second
    // to the admin (the "offered" subject). Order is the dispatch order
    // from `begin`.
    let owner_mail = outbox.iter().find(|e| e.subject.contains("Confirm ownership")).unwrap();
    let admin_mail = outbox.iter().find(|e| e.subject.contains("offered ownership")).unwrap();
    let from_token = extract_transfer_token(&owner_mail.text_body);
    let to_token = extract_transfer_token(&admin_mail.text_body);

    // Owner-side confirm.
    let resp = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/v1/families/{}/transfer-owner/confirm", fam.family_id.into_uuid()))
            .cookie(Cookie::new("access", owner_access))
            .insert_header(("X-Family-Id", fam.family_id.into_uuid().to_string()))
            .set_json(serde_json::json!({ "token": from_token }))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status().as_u16(), 200);

    // Admin-side confirm.
    let admin_access = fresh_access(&stack, &app, &fam.admin_email).await;
    let resp = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/v1/families/{}/transfer-owner/confirm", fam.family_id.into_uuid()))
            .cookie(Cookie::new("access", admin_access))
            .insert_header(("X-Family-Id", fam.family_id.into_uuid().to_string()))
            .set_json(serde_json::json!({ "token": to_token }))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status().as_u16(), 200);

    // Roles swapped on the database.
    let owner_now = stack
        .state
        .memberships
        .find(fam.family_id, fam.owner_id)
        .await
        .expect("owner row")
        .expect("owner still member");
    let admin_now = stack
        .state
        .memberships
        .find(fam.family_id, fam.admin_id)
        .await
        .expect("admin row")
        .expect("admin still member");
    assert_eq!(owner_now.role, Role::Admin);
    assert_eq!(admin_now.role, Role::Owner);

    // `complete / owner_transfer` audit row written exactly once.
    let (rows, _) = stack
        .state
        .audit
        .list_filtered(AuditFilter {
            family_id: fam.family_id,
            from: None,
            to: None,
            action: Some("complete".into()),
            entity_kind: Some("owner_transfer".into()),
            actor_user_id: None,
            page: 1,
            page_size: 50,
        })
        .await
        .expect("audit list");
    assert_eq!(rows.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn confirming_only_one_side_does_not_swap_roles() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let fam = seed_owner_admin_family(&stack, &app, stamp).await;

    let _ = stack.fake_email.drain();
    let owner_access = fresh_access(&stack, &app, &fam.owner_email).await;
    let _ = stack.fake_email.drain();
    let _ = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/v1/families/{}/transfer-owner", fam.family_id.into_uuid()))
            .cookie(Cookie::new("access", owner_access.clone()))
            .insert_header(("X-Family-Id", fam.family_id.into_uuid().to_string()))
            .set_json(serde_json::json!({ "to_user_id": fam.admin_id.into_uuid() }))
            .to_request(),
    )
    .await;

    let outbox = stack.fake_email.drain();
    let owner_mail = outbox.iter().find(|e| e.subject.contains("Confirm ownership")).unwrap();
    let from_token = extract_transfer_token(&owner_mail.text_body);

    let resp = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/v1/families/{}/transfer-owner/confirm", fam.family_id.into_uuid()))
            .cookie(Cookie::new("access", owner_access))
            .insert_header(("X-Family-Id", fam.family_id.into_uuid().to_string()))
            .set_json(serde_json::json!({ "token": from_token }))
            .to_request(),
    )
    .await;
    assert_eq!(resp.status().as_u16(), 200);

    // Roles unchanged.
    let owner_now = stack
        .state
        .memberships
        .find(fam.family_id, fam.owner_id)
        .await
        .expect("owner row")
        .expect("owner still member");
    let admin_now = stack
        .state
        .memberships
        .find(fam.family_id, fam.admin_id)
        .await
        .expect("admin row")
        .expect("admin still member");
    assert_eq!(owner_now.role, Role::Owner);
    assert_eq!(admin_now.role, Role::Admin);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn second_begin_while_pending_returns_409_owner_transfer_pending() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let fam = seed_owner_admin_family(&stack, &app, stamp).await;

    let _ = stack.fake_email.drain();
    let owner_access = fresh_access(&stack, &app, &fam.owner_email).await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{}/transfer-owner", fam.family_id.into_uuid()))
        .cookie(Cookie::new("access", owner_access.clone()))
        .insert_header(("X-Family-Id", fam.family_id.into_uuid().to_string()))
        .set_json(serde_json::json!({ "to_user_id": fam.admin_id.into_uuid() }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);

    // Second begin while the first is still pending -> 409 with the
    // `owner_transfer.pending` slug.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{}/transfer-owner", fam.family_id.into_uuid()))
        .cookie(Cookie::new("access", owner_access))
        .insert_header(("X-Family-Id", fam.family_id.into_uuid().to_string()))
        .set_json(serde_json::json!({ "to_user_id": fam.admin_id.into_uuid() }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 409);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "owner_transfer_pending");
}
