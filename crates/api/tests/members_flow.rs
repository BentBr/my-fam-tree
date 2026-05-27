//! Integration tests for `/families/{family_id}/members[/{user_id}]`.
//!
//! Each test spins up its own `ephemeral_stack` and seeds a small
//! three-role family (owner + admin + user) directly via the repos, so
//! we can exercise the role-matrix gates without going through the
//! invite-accept flow. Sign-in is done via the existing `sign_in`
//! helper so the JWT carries `families` claims that mirror the
//! database — `require_role` only sees what's in the token.

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
use common::{
    ensure_membership, ephemeral_stack, fresh_access, provision_user, seed_three_role_family,
};
use my_family_api::build_app;
use my_family_domain::{AuditFilter, Role};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admin_lists_members_sees_owner_admin_user_in_role_order() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let fam = seed_three_role_family(&stack, &app, stamp).await;

    let access = fresh_access(&stack, &app, &fam.admin_email).await;
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/families/{}/members", fam.family_id.into_uuid()))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", fam.family_id.into_uuid().to_string()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let roles: Vec<&str> = body["data"]["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["role"].as_str().unwrap())
        .collect();
    assert_eq!(roles, vec!["owner", "admin", "user"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admin_promotes_user_to_admin_and_records_audit() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let fam = seed_three_role_family(&stack, &app, stamp).await;

    let access = fresh_access(&stack, &app, &fam.admin_email).await;
    let req = test::TestRequest::patch()
        .uri(&format!(
            "/api/v1/families/{}/members/{}",
            fam.family_id.into_uuid(),
            fam.user_id.into_uuid()
        ))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", fam.family_id.into_uuid().to_string()))
        .set_json(serde_json::json!({ "role": "admin" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["data"]["role"], "admin");

    // Audit row was written with from/to metadata.
    let filter = AuditFilter {
        family_id: fam.family_id,
        from: None,
        to: None,
        action: Some("set_role".into()),
        entity_kind: Some("membership".into()),
        actor_user_id: None,
        page: 1,
        page_size: 50,
    };
    let (rows, _) = stack.state.audit.list_filtered(filter).await.expect("audit list");
    assert!(
        rows.iter().any(|r| r.metadata["to"] == "admin" && r.metadata["from"] == "user"),
        "expected set_role row with from=user, to=admin"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admin_demoting_admin_returns_403() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let fam = seed_three_role_family(&stack, &app, stamp).await;

    // Promote the user → admin via the repo so we have two admin rows.
    let second_admin_email = format!("members-extra-admin-{stamp}@example.com");
    let second_admin_id = provision_user(&stack, &app, &second_admin_email).await;
    ensure_membership(&stack.state, fam.family_id, second_admin_id, Role::Admin).await;

    let access = fresh_access(&stack, &app, &fam.admin_email).await;
    let req = test::TestRequest::patch()
        .uri(&format!(
            "/api/v1/families/{}/members/{}",
            fam.family_id.into_uuid(),
            second_admin_id.into_uuid()
        ))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", fam.family_id.into_uuid().to_string()))
        .set_json(serde_json::json!({ "role": "user" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 403);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["code"], "family_insufficient_role");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owner_revoking_user_removes_membership_and_audits() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let fam = seed_three_role_family(&stack, &app, stamp).await;

    let access = fresh_access(&stack, &app, &fam.owner_email).await;
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1/families/{}/members/{}",
            fam.family_id.into_uuid(),
            fam.user_id.into_uuid()
        ))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", fam.family_id.into_uuid().to_string()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);

    // Row is gone.
    assert!(
        stack.state.memberships.find(fam.family_id, fam.user_id).await.expect("find").is_none(),
        "membership row should have been removed"
    );

    // Audit row was written.
    let filter = AuditFilter {
        family_id: fam.family_id,
        from: None,
        to: None,
        action: Some("remove".into()),
        entity_kind: Some("membership".into()),
        actor_user_id: None,
        page: 1,
        page_size: 50,
    };
    let (rows, _) = stack.state.audit.list_filtered(filter).await.expect("audit list");
    assert!(!rows.is_empty(), "expected a remove/membership audit row");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admin_cannot_revoke_admin_returns_403() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let fam = seed_three_role_family(&stack, &app, stamp).await;

    let second_admin_email = format!("members-extra-admin-revoke-{stamp}@example.com");
    let second_admin_id = provision_user(&stack, &app, &second_admin_email).await;
    ensure_membership(&stack.state, fam.family_id, second_admin_id, Role::Admin).await;

    let access = fresh_access(&stack, &app, &fam.admin_email).await;
    let req = test::TestRequest::delete()
        .uri(&format!(
            "/api/v1/families/{}/members/{}",
            fam.family_id.into_uuid(),
            second_admin_id.into_uuid()
        ))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", fam.family_id.into_uuid().to_string()))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 403);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nobody_can_target_self_even_with_owner_role() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let fam = seed_three_role_family(&stack, &app, stamp).await;

    let owner_user =
        stack.state.users.find_by_email(&fam.owner_email).await.expect("lookup").expect("user");

    let access = fresh_access(&stack, &app, &fam.owner_email).await;
    let req = test::TestRequest::patch()
        .uri(&format!(
            "/api/v1/families/{}/members/{}",
            fam.family_id.into_uuid(),
            owner_user.id.into_uuid()
        ))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", fam.family_id.into_uuid().to_string()))
        .set_json(serde_json::json!({ "role": "admin" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 403);

    // The mutation didn't go through — owner row is intact.
    let still_owner = stack
        .state
        .memberships
        .find(fam.family_id, owner_user.id)
        .await
        .expect("find")
        .expect("owner membership exists");
    assert_eq!(still_owner.role, Role::Owner);

    // Suppress unused warning for the fields not exercised in this test.
    let _ = (fam.admin_id, fam.user_email);
}
