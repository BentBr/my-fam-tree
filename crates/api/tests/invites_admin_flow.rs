//! Phase D — invite admin side: create-with-person (email + audit),
//! pending-invite cancel, and duplicate rejection.
//! (The accept side lives in `invites_flow.rs`.)
//!
//! Each test spins its own `ephemeral_stack` and walks the HTTP surface
//! end-to-end so the audit + email side-effects are exercised together with
//! the persistence path.

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
use common::ephemeral_stack;
use common::invites::seed_admin_family_with_person;
use my_fam_tree_api::build_app;
use my_fam_tree_domain::{AuditFilter, FamilyId};
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admin_creates_invite_with_person_id_emits_email_and_audit() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let (access, family_id_str, klaus_id) =
        seed_admin_family_with_person(&stack, &app, stamp).await;
    let family_uuid: Uuid = family_id_str.parse().unwrap();
    let family_id = FamilyId::from_uuid(family_uuid);

    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id_str}/invites"))
        .cookie(Cookie::new("access", access))
        .set_json(serde_json::json!({
            "email": format!("newbie-{stamp}@example.com"),
            "role": "user",
            "person_id": klaus_id.into_uuid(),
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200, "invite POST should succeed");

    // One outbound email captured, addressed to the invitee.
    let captured = stack.fake_email.drain();
    assert_eq!(captured.len(), 1, "exactly one invite email");
    assert_eq!(captured[0].to_addr, format!("newbie-{stamp}@example.com"));

    // Audit row exists with entity_id = invite_id and metadata.person_id.
    let filter = AuditFilter {
        family_id,
        from: None,
        to: None,
        action: Some("invite".into()),
        entity_kind: Some("membership".into()),
        actor_user_id: None,
        page: 1,
        page_size: 50,
    };
    let (rows, _) = stack.state.audit.list_filtered(filter).await.expect("audit list");
    assert!(!rows.is_empty(), "expected an invite/membership audit row");
    let row = &rows[0];
    assert!(row.entity_id.is_some(), "invite audit row must carry entity_id");
    assert_eq!(row.metadata["person_id"], serde_json::json!(klaus_id.into_uuid()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_invite_removes_row_and_audits() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let (access, family_id_str, _klaus_id) =
        seed_admin_family_with_person(&stack, &app, stamp).await;
    let family_uuid: Uuid = family_id_str.parse().unwrap();
    let family_id = FamilyId::from_uuid(family_uuid);

    // Create an invite to cancel.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id_str}/invites"))
        .cookie(Cookie::new("access", access.clone()))
        .set_json(serde_json::json!({
            "email": format!("cancel-{stamp}@example.com"),
            "role": "user",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200);

    // List to fetch the id.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/families/{family_id_str}/invites"))
        .cookie(Cookie::new("access", access.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let invite_id = body["data"]["data"][0]["id"].as_str().expect("invite id in list").to_string();

    // DELETE it.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/families/{family_id_str}/invites/{invite_id}"))
        .cookie(Cookie::new("access", access))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200, "cancel should succeed");

    // Row is gone.
    let remaining =
        stack.state.invites.list_pending_for_family(family_id).await.expect("list pending");
    assert!(remaining.is_empty(), "no pending invites after cancel");

    // Audit row written with (cancel, invite).
    let filter = AuditFilter {
        family_id,
        from: None,
        to: None,
        action: Some("cancel".into()),
        entity_kind: Some("invite".into()),
        actor_user_id: None,
        page: 1,
        page_size: 50,
    };
    let (rows, _) = stack.state.audit.list_filtered(filter).await.expect("audit list");
    assert_eq!(rows.len(), 1, "expected one cancel/invite row");
    let invite_uuid: Uuid = invite_id.parse().unwrap();
    assert_eq!(rows[0].entity_id, Some(invite_uuid));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn duplicate_invite_returns_409_invite_duplicate() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let (access, family_id_str, _klaus_id) =
        seed_admin_family_with_person(&stack, &app, stamp).await;

    let email = format!("dup-{stamp}@example.com");

    // First invite succeeds.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id_str}/invites"))
        .cookie(Cookie::new("access", access.clone()))
        .set_json(serde_json::json!({ "email": email, "role": "user" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200);

    // Second invite for the same email returns 409 invite_duplicate.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id_str}/invites"))
        .cookie(Cookie::new("access", access))
        .set_json(serde_json::json!({ "email": email, "role": "user" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 409);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "invite_duplicate");
}
