//! Security-audit regressions for token binding: invite accept,
//! owner-transfer confirm and email-change confirm validate the actor
//! BEFORE consuming the token, and refresh-token reuse revokes every
//! session. See `security_regressions.rs` for the audit overview.

#![expect(
    clippy::expect_used,
    clippy::shadow_unrelated,
    clippy::shadow_reuse,
    reason = "test code: setup failures abort the test with context; test steps rebind the same response/body names"
)]

mod common;

use actix_web::cookie::Cookie;
use actix_web::test;
use common::{create_family, ephemeral_stack, sign_in};
use my_fam_tree_api::build_app;
use my_fam_tree_domain::{FamilyId, Role};
use uuid::Uuid;

/// Pull `token` out of a `…?token=XYZ` URL in an email body. Mirrors the
/// helper used by the invite / owner-transfer flow tests.
fn token_from_link(body: &str) -> String {
    let after = body.split("token=").nth(1).expect("token= present");
    after.split(|c: char| c.is_whitespace() || c == '"').next().expect("token chars").to_owned()
}

// ---------------------------------------------------------------------------
// FIX #2: invite accept validates the session email BEFORE consuming. A
// wrong signed-in user gets 422 and the invite STAYS pending so the rightful
// recipient can still accept it.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invite_accept_wrong_session_does_not_burn_token() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());

    // Admin/owner creates a family and an invite for `invitee`.
    let (owner_access, _r) =
        sign_in(&stack, &app, &format!("inv2-owner-{stamp}@example.com")).await;
    let (owner_access, family_id_str) =
        create_family(&app, &owner_access, &format!("Inv2-{stamp}")).await;
    let family_id = FamilyId::from_uuid(family_id_str.parse::<Uuid>().unwrap());

    let invitee_email = format!("inv2-invitee-{stamp}@example.com");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id_str}/invites"))
        .cookie(Cookie::new("access", owner_access))
        .set_json(serde_json::json!({ "email": invitee_email, "role": "user" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200);
    let invite_token = {
        let captured = stack.fake_email.drain();
        let mail = captured.iter().find(|m| m.to_addr == invitee_email).expect("invite email");
        token_from_link(&mail.text_body)
    };

    // A DIFFERENT signed-in user (neither the inviter nor the invitee) tries
    // to accept. This must 422 and NOT consume the invite.
    let (other_access, _r) =
        sign_in(&stack, &app, &format!("inv2-other-{stamp}@example.com")).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/invites/accept")
        .cookie(Cookie::new("access", other_access))
        .set_json(serde_json::json!({ "token": invite_token.clone() }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 422, "wrong session must 422");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "validation_failed");

    // The invite is STILL pending after the rejected attempt.
    let pending = stack.state.invites.list_pending_for_family(family_id).await.expect("pending");
    assert_eq!(pending.len(), 1, "invite must remain pending after wrong-session 422");

    // The rightful invitee can now accept it → membership inserted.
    let (invitee_access, _r) = sign_in(&stack, &app, &invitee_email).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/invites/accept")
        .cookie(Cookie::new("access", invitee_access))
        .set_json(serde_json::json!({ "token": invite_token }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200, "rightful invitee must still be able to accept");

    let invitee = stack
        .state
        .users
        .find_by_email(&invitee_email)
        .await
        .expect("lookup")
        .expect("invitee user");
    let m = stack.state.memberships.find(family_id, invitee.id).await.expect("find membership");
    assert!(m.is_some(), "membership inserted for the rightful invitee");
}

// ---------------------------------------------------------------------------
// FIX #3: owner-transfer confirm binds to the authenticated user. A third
// authed user presenting a leaked transfer token gets 403 and the side's
// `*_confirmed_at` is unchanged.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owner_transfer_confirm_rejects_wrong_actor_without_burning_side() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());

    // Owner + admin on one family.
    let owner_email = format!("xfer3-owner-{stamp}@example.com");
    let admin_email = format!("xfer3-admin-{stamp}@example.com");
    let (owner_access, _r) = sign_in(&stack, &app, &owner_email).await;
    let (owner_access, family_id_str) =
        create_family(&app, &owner_access, &format!("Xfer3-{stamp}")).await;
    let family_id = FamilyId::from_uuid(family_id_str.parse::<Uuid>().unwrap());

    drop(sign_in(&stack, &app, &admin_email).await);
    let admin_id =
        stack.state.users.find_by_email(&admin_email).await.expect("lookup").expect("admin").id;
    stack.state.memberships.insert(family_id, admin_id, Role::Admin).await.expect("admin member");

    // Owner begins the transfer (two emails dispatched). The access cookie
    // from `create_family` already carries the owner role.
    drop(stack.fake_email.drain());
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id_str}/transfer-owner"))
        .cookie(Cookie::new("access", owner_access))
        .insert_header(("X-Family-Id", family_id_str.clone()))
        .set_json(serde_json::json!({ "to_user_id": admin_id.into_uuid() }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 200);
    let outbox = stack.fake_email.drain();
    let confirm_mail = outbox.iter().find(|e| e.subject.contains("Confirm ownership")).unwrap();
    let from_token = token_from_link(&confirm_mail.text_body);

    // A THIRD authed user (neither from nor to) presents the owner-side token.
    let (third_access, _r) =
        sign_in(&stack, &app, &format!("xfer3-third-{stamp}@example.com")).await;
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id_str}/transfer-owner/confirm"))
        .cookie(Cookie::new("access", third_access))
        .insert_header(("X-Family-Id", family_id_str.clone()))
        .set_json(serde_json::json!({ "token": from_token }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 403, "wrong actor must 403");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "family_insufficient_role");

    // The from-side confirmation is UNCHANGED.
    let active = stack
        .state
        .owner_transfers
        .find_active(family_id)
        .await
        .expect("find active")
        .expect("transfer still pending");
    assert!(active.from_confirmed_at.is_none(), "from side must NOT be confirmed by wrong actor");
    assert!(active.to_confirmed_at.is_none());
}

// ---------------------------------------------------------------------------
// FIX #5: email-change confirm validates purpose + owner BEFORE consuming. A
// wrong authed user presenting another user's email-change token gets 401 and
// the token stays consumable by the rightful owner.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn email_change_confirm_wrong_user_does_not_burn_token() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());

    // Victim requests an email change → magic link sent to their old address.
    let victim_email = format!("ec5-victim-{stamp}@example.com");
    let (victim_access, _r) = sign_in(&stack, &app, &victim_email).await;
    stack.fake_email.drain();
    let new_email = format!("ec5-new-{stamp}@example.com");
    let req = test::TestRequest::post()
        .uri("/api/v1/users/me/email-change")
        .cookie(Cookie::new("access", victim_access.clone()))
        .set_json(serde_json::json!({ "new_email": new_email }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let token = {
        let captured = stack.fake_email.drain();
        let mail = captured.last().expect("email-change email");
        token_from_link(&mail.text_body)
    };

    // A DIFFERENT authed user tries to confirm the victim's token.
    let (attacker_access, _r) =
        sign_in(&stack, &app, &format!("ec5-attacker-{stamp}@example.com")).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/users/me/email-change/confirm")
        .cookie(Cookie::new("access", attacker_access))
        .set_json(serde_json::json!({ "token": token.clone() }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401, "wrong user must 401");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_magic_link_invalid");

    // The rightful owner can STILL confirm — the token wasn't burned.
    let req = test::TestRequest::post()
        .uri("/api/v1/users/me/email-change/confirm")
        .cookie(Cookie::new("access", victim_access))
        .set_json(serde_json::json!({ "token": token }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200, "rightful owner must still confirm the still-valid token");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["email"], new_email);
}

// ---------------------------------------------------------------------------
// FIX #6: refresh-token reuse detection. After one rotation the old token is
// revoked; re-presenting it 401s AND revokes every session — so the NEW
// (previously valid) token then also fails.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refresh_token_reuse_revokes_all_sessions() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());

    let (_access, refresh_old) =
        sign_in(&stack, &app, &format!("reuse6-{stamp}@example.com")).await;

    // First refresh: rotates. We capture the NEW refresh cookie and the old
    // one is now revoked.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .cookie(Cookie::new("refresh", refresh_old.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200, "first refresh rotates");
    let refresh_new = res
        .response()
        .cookies()
        .find(|c| c.name() == "refresh")
        .expect("rotated refresh cookie")
        .value()
        .to_owned();
    assert_ne!(refresh_new, refresh_old, "rotation must mint a new token");

    // Re-present the OLD (now revoked) token → reuse detected → 401.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .cookie(Cookie::new("refresh", refresh_old))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401, "reused old token must 401");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_refresh_invalid");

    // The reuse handler revoked ALL sessions, so the NEW token (which WAS
    // valid until now) must now also fail.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/refresh")
        .cookie(Cookie::new("refresh", refresh_new))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401, "all sessions revoked: the new token must also fail");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "auth_refresh_invalid");
}
