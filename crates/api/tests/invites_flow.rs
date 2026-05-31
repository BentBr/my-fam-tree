//! Phase D — invite-from-PersonDetail + accept-side `linked_user_id` wiring
//! + pending-invite list / cancel.
//!
//! Each test spins its own `ephemeral_stack` and walks the HTTP surface
//! end-to-end so the audit + email side-effects are exercised together with
//! the persistence path. The regex helper at the top pulls the
//! `invite/accept?token=…` value out of the captured email body so the
//! accept arm can be exercised in the same scope.

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
use common::{ephemeral_stack, extract_token_from_link, sign_in};
use my_fam_tree_api::build_app;
use my_fam_tree_domain::{AuditFilter, FamilyId, PersonDraft, PersonId};
use uuid::Uuid;

/// Seed an admin-owned family with one person (Klaus) and return
/// `(access_cookie, family_uuid_string, klaus_person_id)` for the caller
/// to drive the invite flow from.
async fn seed_admin_family_with_person<S, B>(
    stack: &common::TestStack,
    app: &S,
    stamp: u128,
) -> (String, String, PersonId)
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let owner_email = format!("invite-owner-{stamp}@example.com");
    let (access, _r) = sign_in(stack, app, &owner_email).await;
    let (access, family_id_str) =
        common::create_family(app, &access, &format!("InviteFam-{stamp}")).await;
    let family_uuid: Uuid = family_id_str.parse().expect("uuid");
    let family_id = FamilyId::from_uuid(family_uuid);

    // Drain the magic-link email so callers can inspect invite mails by
    // grabbing `.last()` on the captured drain.
    stack.fake_email.drain();

    let klaus = stack
        .state
        .persons
        .create(
            family_id,
            PersonDraft {
                given_name: "Klaus".into(),
                family_name: "Müller".into(),
                ..PersonDraft::default()
            },
        )
        .await
        .expect("create klaus");

    (access, family_id_str, klaus.id)
}

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
async fn accept_links_user_to_person_when_invite_carries_person_id() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let (access, family_id_str, klaus_id) =
        seed_admin_family_with_person(&stack, &app, stamp).await;
    let family_uuid: Uuid = family_id_str.parse().unwrap();
    let family_id = FamilyId::from_uuid(family_uuid);

    let invitee_email = format!("linked-{stamp}@example.com");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id_str}/invites"))
        .cookie(Cookie::new("access", access))
        .set_json(serde_json::json!({
            "email": invitee_email,
            "role": "user",
            "person_id": klaus_id.into_uuid(),
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200);

    // Pull the invite token out of the captured email. There may be other
    // emails in the queue, so we match by recipient address.
    let captured = stack.fake_email.drain();
    let invite_mail =
        captured.iter().find(|m| m.to_addr == invitee_email).expect("invite email captured");
    let invite_token = extract_token_from_link(&invite_mail.text_body);

    // Invitee signs in via magic link.
    let (invitee_access, _r) = sign_in(&stack, &app, &invitee_email).await;
    let invitee_user = stack
        .state
        .users
        .find_by_email(&invitee_email)
        .await
        .expect("user lookup")
        .expect("invitee user");

    // POST /invites/accept → 200 + linked.
    let req = test::TestRequest::post()
        .uri("/api/v1/invites/accept")
        .cookie(Cookie::new("access", invitee_access))
        .set_json(serde_json::json!({ "token": invite_token }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200, "accept should succeed");

    // Klaus's row now points at the invitee.
    let klaus = stack
        .state
        .persons
        .find_in_family(family_id, klaus_id)
        .await
        .expect("find klaus")
        .expect("klaus row");
    assert_eq!(klaus.linked_user_id, Some(invitee_user.id), "linked_user_id wired");

    // Audit log carries verify + accept_invite rows tied to this invite.
    let filter = AuditFilter {
        family_id,
        from: None,
        to: None,
        action: None,
        entity_kind: None,
        actor_user_id: None,
        page: 1,
        page_size: 50,
    };
    let (rows, _) = stack.state.audit.list_filtered(filter).await.expect("audit list");
    assert!(
        rows.iter().any(|r| r.action == "verify" && r.entity_kind == "invite"),
        "expected (verify, invite) row"
    );
    assert!(
        rows.iter().any(|r| r.action == "accept_invite" && r.entity_kind == "membership"),
        "expected (accept_invite, membership) row"
    );
}

/// A second invite-accept for a user who already has a membership in
/// this family is idempotent on the membership row AND still runs the
/// person-link side-effect. Two scenarios this covers:
///   - a user re-clicking the same invite link (sessionStorage dedup
///     covers a tight double-mount; this catches the slower paths);
///   - a follow-up person-targeted invite to a member who already
///     has a family-level membership — the invite-accept handler
///     no-ops the membership insert and wires `persons.linked_user_id`
///     for the bound person.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn second_accept_is_idempotent_on_membership_and_still_wires_person_link() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let (owner_access, family_id_str, klaus_id) =
        seed_admin_family_with_person(&stack, &app, stamp).await;
    let family_uuid: Uuid = family_id_str.parse().unwrap();
    let family_id = FamilyId::from_uuid(family_uuid);

    let invitee_email = format!("idem-{stamp}@example.com");
    let (invitee_access, _r) = sign_in(&stack, &app, &invitee_email).await;
    let invitee_user = stack
        .state
        .users
        .find_by_email(&invitee_email)
        .await
        .expect("user lookup")
        .expect("invitee user");

    // First invite: plain family-level, no person_id. Invitee accepts → has
    // a `user` membership.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id_str}/invites"))
        .cookie(Cookie::new("access", owner_access.clone()))
        .set_json(serde_json::json!({
            "email": invitee_email,
            "role": "user",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200);
    let first_token = {
        let captured = stack.fake_email.drain();
        let mail = captured.iter().find(|m| m.to_addr == invitee_email).expect("first email");
        extract_token_from_link(&mail.text_body)
    };
    let req = test::TestRequest::post()
        .uri("/api/v1/invites/accept")
        .cookie(Cookie::new("access", invitee_access.clone()))
        .set_json(serde_json::json!({ "token": first_token }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200, "first accept should succeed");

    // Sanity: membership exists.
    let m =
        stack.state.memberships.find(family_id, invitee_user.id).await.expect("memberships.find");
    assert!(m.is_some(), "membership should exist after first accept");

    // Second invite: person-targeted, same email, same role. Without
    // `ON CONFLICT DO NOTHING` on `memberships.insert`, the accept below
    // would 500 here.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id_str}/invites"))
        .cookie(Cookie::new("access", owner_access))
        .set_json(serde_json::json!({
            "email": invitee_email,
            "role": "user",
            "person_id": klaus_id.into_uuid(),
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200, "second invite POST should succeed");
    let second_token = {
        let captured = stack.fake_email.drain();
        let mail = captured.iter().find(|m| m.to_addr == invitee_email).expect("second email");
        extract_token_from_link(&mail.text_body)
    };
    let req = test::TestRequest::post()
        .uri("/api/v1/invites/accept")
        .cookie(Cookie::new("access", invitee_access))
        .set_json(serde_json::json!({ "token": second_token }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200, "second accept must be idempotent on membership");

    // The whole point: the person-link side-effect still runs, even though
    // the membership insert was a no-op. Klaus is now linked to invitee.
    let klaus = stack
        .state
        .persons
        .find_in_family(family_id, klaus_id)
        .await
        .expect("find klaus")
        .expect("klaus row");
    assert_eq!(
        klaus.linked_user_id,
        Some(invitee_user.id),
        "second accept must wire person.linked_user_id"
    );
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn anonymous_accept_creates_user_and_signs_in() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let (access, family_id_str, klaus_id) =
        seed_admin_family_with_person(&stack, &app, stamp).await;

    let invitee_email = format!("anon-accept-{stamp}@example.com");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id_str}/invites"))
        .cookie(Cookie::new("access", access))
        .set_json(serde_json::json!({
            "email": invitee_email,
            "role": "user",
            "person_id": klaus_id.into_uuid(),
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200);

    let captured = stack.fake_email.drain();
    let mail = captured.iter().find(|m| m.to_addr == invitee_email).expect("invite email");
    let invite_token = extract_token_from_link(&mail.text_body);

    // Pre-condition: the invitee does not exist yet.
    assert!(
        stack.state.users.find_by_email(&invitee_email).await.expect("lookup").is_none(),
        "invitee should not exist before the accept call"
    );

    // POST /invites/accept WITHOUT a session cookie. The BE creates the
    // user, accepts the invite, and issues an access cookie inline.
    let req = test::TestRequest::post()
        .uri("/api/v1/invites/accept")
        .set_json(serde_json::json!({ "token": invite_token }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200, "anonymous accept should succeed");

    // The access cookie is set in the response so the browser is signed in.
    let cookies = res.response().cookies().collect::<Vec<_>>();
    assert!(
        cookies.iter().any(|c| c.name() == "access" && !c.value().is_empty()),
        "anonymous accept response must include an access cookie"
    );

    // The user now exists; Klaus is linked to them.
    let invitee = stack
        .state
        .users
        .find_by_email(&invitee_email)
        .await
        .expect("lookup")
        .expect("user was created by accept");
    let family_id = FamilyId::from_uuid(family_id_str.parse::<Uuid>().unwrap());
    let klaus = stack
        .state
        .persons
        .find_in_family(family_id, klaus_id)
        .await
        .expect("find klaus")
        .expect("klaus row");
    assert_eq!(klaus.linked_user_id, Some(invitee.id), "linked_user_id wired to new user");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn signed_in_mismatched_email_returns_invite_email_mismatch() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let (access, family_id_str, _klaus_id) =
        seed_admin_family_with_person(&stack, &app, stamp).await;

    let invitee_email = format!("mismatch-target-{stamp}@example.com");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id_str}/invites"))
        .cookie(Cookie::new("access", access))
        .set_json(serde_json::json!({ "email": invitee_email, "role": "user" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200);

    let captured = stack.fake_email.drain();
    let mail = captured.iter().find(|m| m.to_addr == invitee_email).expect("invite email");
    let invite_token = extract_token_from_link(&mail.text_body);

    // A third party signs in (NOT the invitee, NOT the inviting admin)
    // and tries to claim the invite.
    let other_email = format!("not-the-invitee-{stamp}@example.com");
    let (other_access, _r) = sign_in(&stack, &app, &other_email).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/invites/accept")
        .cookie(Cookie::new("access", other_access))
        .set_json(serde_json::json!({ "token": invite_token }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 422, "mismatched email must 422");

    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "validation_failed");
    let fields = body["fields"].as_array().expect("fields array present");
    assert!(
        fields.iter().any(|f| f["code"] == "validation.invite_email_mismatch"),
        "expected validation.invite_email_mismatch violation in the 422 body"
    );
}
