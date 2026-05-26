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
use common::{ephemeral_stack, sign_in};
use my_family_api::{AppState, build_app};
use my_family_domain::{AuditFilter, FamilyId, Role, UserId};
use uuid::Uuid;

/// Sign `email` in (which provisions the `users` row + a magic-link
/// session) and return the resulting `UserId`. The returned cookies
/// are discarded — the caller signs in again after memberships are
/// inserted so the fresh JWT carries the right `families` claim.
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

/// Insert a membership row directly via the repo. Uses `set_role` if
/// the row already exists (the owner-create path may have written one
/// for the family creator).
async fn ensure_membership(state: &AppState, family_id: FamilyId, user_id: UserId, role: Role) {
    if state.memberships.find(family_id, user_id).await.expect("find").is_some() {
        state.memberships.set_role(family_id, user_id, role).await.expect("set_role");
    } else {
        state.memberships.insert(family_id, user_id, role).await.expect("insert");
    }
}

/// Helper bundling the three users for a freshly-seeded family.
struct ThreeRoleFamily {
    family_id: FamilyId,
    owner_email: String,
    admin_email: String,
    user_email: String,
    admin_id: UserId,
    user_id: UserId,
}

/// Seed a fresh family with an owner, an admin and a user. Returns the
/// emails so callers can sign in as any role.
///
/// Implementation: the first `create_family` call mints the owner row
/// for free; we then provision two more users via magic-link sign-in
/// and bolt them on at the requested role.
async fn seed_three_role_family<S, B>(
    stack: &common::TestStack,
    app: &S,
    stamp: u128,
) -> ThreeRoleFamily
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let owner_email = format!("members-owner-{stamp}@example.com");
    let admin_email = format!("members-admin-{stamp}@example.com");
    let user_email = format!("members-user-{stamp}@example.com");

    // Owner signs in + creates the family (joins as Owner).
    let (owner_access, _r) = sign_in(stack, app, &owner_email).await;
    let (_owner_access, family_id_str) =
        common::create_family(app, &owner_access, &format!("MembersFam-{stamp}")).await;
    let family_uuid: Uuid = family_id_str.parse().expect("uuid");
    let family_id = FamilyId::from_uuid(family_uuid);

    // Provision the admin + user identities and bolt them on at the
    // appropriate role.
    let admin_id = provision_user(stack, app, &admin_email).await;
    let user_id = provision_user(stack, app, &user_email).await;
    ensure_membership(&stack.state, family_id, admin_id, Role::Admin).await;
    ensure_membership(&stack.state, family_id, user_id, Role::User).await;

    ThreeRoleFamily { family_id, owner_email, admin_email, user_email, admin_id, user_id }
}

/// Re-sign-in helper that returns the access cookie value after a
/// membership change so the JWT carries the new role.
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
