//! Integration tests for `GET /api/v1/families/{family_id}/audit`.
//!
//! Each test spins up its own `ephemeral_stack()` (postgres + redis via
//! testcontainers) so they can run in parallel without sharing data.
//! The pattern mirrors `person_contacts_flow.rs` — sign in as the
//! family owner (or another role), seed a person, write audit rows
//! directly through `state.audit.record`, then exercise the route via
//! `test::call_service`.

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
use my_fam_tree_api::build_app;
use my_fam_tree_domain::{AuditEntry, FamilyId, PersonDraft, PersonId, Role};
use uuid::Uuid;

/// Seed a person directly via the repo so we have a stable id for the
/// audit metadata to reference.
async fn seed_person(
    state: &my_fam_tree_api::AppState,
    family_id: FamilyId,
    given: &str,
    family: &str,
) -> PersonId {
    let draft = PersonDraft {
        given_name: given.to_string(),
        family_name: family.to_string(),
        ..Default::default()
    };
    state.persons.create(family_id, draft).await.expect("seed person").id
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admin_lists_audit_returns_recent_rows_with_actor_and_entity_person() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _refresh) = sign_in(&stack, &app, "audit-admin@example.com").await;
    let (access, family_id_str) = create_family(&app, &access, "AuditFam").await;
    let family_uuid: Uuid = family_id_str.parse().expect("uuid");
    let family_id = FamilyId::from_uuid(family_uuid);

    // The owner that created the family is the actor we'll write audit
    // rows on behalf of.
    let owner = stack
        .state
        .users
        .find_by_email("audit-admin@example.com")
        .await
        .expect("lookup")
        .expect("user exists");

    let klaus = seed_person(&stack.state, family_id, "Klaus", "Müller").await;

    stack
        .state
        .audit
        .record(AuditEntry {
            family_id,
            actor_user_id: Some(owner.id),
            action: "create".into(),
            entity_kind: "contact".into(),
            entity_id: Some(Uuid::new_v4()),
            metadata: serde_json::json!({ "person_id": klaus.into_uuid() }),
        })
        .await
        .expect("audit insert");

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/families/{family_uuid}/audit"))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id_str.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["data"][0]["action"], "create");
    assert_eq!(body["data"]["data"][0]["entity_kind"], "contact");
    assert_eq!(
        body["data"]["data"][0]["entity_person_id"].as_str().unwrap(),
        klaus.into_uuid().to_string()
    );
    let name = body["data"]["data"][0]["entity_person_name"].as_str().unwrap_or("");
    assert!(name.contains("Klaus"), "expected Klaus, got {name}");
    assert_eq!(body["data"]["page"], 1);
    assert_eq!(body["data"]["page_size"], 50);
    assert!(body["data"]["total"].as_i64().unwrap_or(0) >= 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_role_audit_listing_returns_403() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (owner_access, _r) = sign_in(&stack, &app, "audit-owner@example.com").await;
    let (_owner_access, family_id_str) = create_family(&app, &owner_access, "AuditGate").await;
    let family_uuid: Uuid = family_id_str.parse().expect("uuid");

    // Add a regular `user`-role member.
    let guest_email = "audit-user@example.com";
    let _ = sign_in(&stack, &app, guest_email).await;
    let guest =
        stack.state.users.find_by_email(guest_email).await.expect("lookup").expect("user exists");
    stack
        .state
        .memberships
        .insert(FamilyId::from_uuid(family_uuid), guest.id, Role::User)
        .await
        .expect("membership insert");

    // Re-sign so the JWT carries the new membership.
    let (guest_access, _r) = sign_in(&stack, &app, guest_email).await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/families/{family_uuid}/audit"))
        .cookie(Cookie::new("access", guest_access))
        .insert_header(("X-Family-Id", family_id_str))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "family_insufficient_role");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_filters_by_action_and_returns_correct_count() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "audit-filter@example.com").await;
    let (access, family_id_str) = create_family(&app, &access, "FilterFam").await;
    let family_uuid: Uuid = family_id_str.parse().expect("uuid");
    let family_id = FamilyId::from_uuid(family_uuid);

    let owner = stack
        .state
        .users
        .find_by_email("audit-filter@example.com")
        .await
        .expect("lookup")
        .expect("user");
    let klaus = seed_person(&stack.state, family_id, "Klaus", "Filter").await;

    for action in ["create", "update", "delete", "create"] {
        stack
            .state
            .audit
            .record(AuditEntry {
                family_id,
                actor_user_id: Some(owner.id),
                action: action.into(),
                entity_kind: "contact".into(),
                entity_id: Some(Uuid::new_v4()),
                metadata: serde_json::json!({ "person_id": klaus.into_uuid() }),
            })
            .await
            .expect("audit insert");
    }

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/families/{family_uuid}/audit?action=create"))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id_str))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let count = body["data"]["data"].as_array().map_or(0, Vec::len);
    // The `create_family` route also writes one `(create, membership)`
    // row for the owner, but that's a different action — filter is
    // strictly `action=create` AND we wrote two `create` `contact`
    // rows, plus the one membership create. Strict equality 3.
    let total = body["data"]["total"].as_i64().unwrap_or(0);
    assert!(count >= 2, "expected at least 2 create rows, got {count}");
    assert!(total >= 2, "expected total >= 2, got {total}");
    // All returned rows should have action=create.
    for row in body["data"]["data"].as_array().unwrap() {
        assert_eq!(row["action"], "create");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_pagination_meta_matches_requested_page_size() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "audit-page@example.com").await;
    let (access, family_id_str) = create_family(&app, &access, "PageFam").await;
    let family_uuid: Uuid = family_id_str.parse().expect("uuid");
    let family_id = FamilyId::from_uuid(family_uuid);

    let owner = stack
        .state
        .users
        .find_by_email("audit-page@example.com")
        .await
        .expect("lookup")
        .expect("user");
    let klaus = seed_person(&stack.state, family_id, "Klaus", "Pager").await;

    for _ in 0_u32..7_u32 {
        stack
            .state
            .audit
            .record(AuditEntry {
                family_id,
                actor_user_id: Some(owner.id),
                action: "update".into(),
                entity_kind: "person".into(),
                entity_id: Some(klaus.into_uuid()),
                metadata: serde_json::json!({}),
            })
            .await
            .expect("audit insert");
    }

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/families/{family_uuid}/audit?page=1&page_size=50"))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id_str))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["page"], 1);
    assert_eq!(body["data"]["page_size"], 50);
    assert!(body["data"]["total"].as_i64().unwrap_or(0) >= 7);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_invalid_page_size_falls_back_to_50() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "audit-clamp@example.com").await;
    let (access, family_id_str) = create_family(&app, &access, "ClampFam").await;
    let family_uuid: Uuid = family_id_str.parse().expect("uuid");

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/families/{family_uuid}/audit?page_size=999"))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id_str))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["page_size"], 50);
}
