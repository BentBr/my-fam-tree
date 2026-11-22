//! Every expiry decision and every "accepted / consumed / revoked at"
//! timestamp follows the injected `AppState.clock`, not system time or
//! Postgres `now()`. The clock is pinned to 2040 — far from real time — so
//! any path still on the wall clock or SQL `now()` fails visibly.

#![expect(
    clippy::shadow_unrelated,
    reason = "test code: test steps rebind the same response/body names"
)]

mod common;

use actix_web::cookie::Cookie;
use actix_web::test;
use common::invites::seed_admin_family_with_person;
use common::{ephemeral_stack, extract_token_from_link, sign_in};
use my_fam_tree_api::auth::{generate_opaque_token, hash_token};
use my_fam_tree_api::build_app;
use my_fam_tree_domain::{FamilyId, MagicLinkPurpose};
use time::macros::datetime;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

const T: OffsetDateTime = datetime!(2040-01-01 00:00 UTC);

/// `invite_ttl_seconds` / `ttl_seconds` / `access_ttl_seconds` in the harness config.
const INVITE_TTL: Duration = Duration::seconds(1_209_600);
const MAGIC_TTL: Duration = Duration::seconds(900);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invite_flow_follows_injected_clock() {
    let stack = ephemeral_stack().await;
    stack.clock.set(T);
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let (access, family_id_str, _klaus) = seed_admin_family_with_person(&stack, &app, stamp).await;
    let family_id = FamilyId::from_uuid(family_id_str.parse::<Uuid>().expect("uuid"));

    let invite = |email: String| {
        test::TestRequest::post()
            .uri(&format!("/api/v1/families/{family_id_str}/invites"))
            .cookie(Cookie::new("access", access.clone()))
            .set_json(serde_json::json!({ "email": email, "role": "user" }))
            .to_request()
    };

    // Created at T → expires at T + invite TTL; listed as pending at T.
    let first = format!("clock-a-{stamp}@example.com");
    let res = test::call_service(&app, invite(first.clone())).await;
    assert_eq!(res.status().as_u16(), 200, "create invite at T");
    let pending = stack.state.invites.list_pending_for_family(family_id, T).await.expect("list");
    let row = pending.iter().find(|i| i.email == first).expect("first invite pending at T");
    assert_eq!(row.expires_at, T + INVITE_TTL, "expires_at derives from the injected clock");

    // Accept at T stamps accepted_at = T.
    let mail = stack.fake_email.drain();
    let token = extract_token_from_link(
        &mail.iter().find(|m| m.to_addr == first).expect("invite mail").text_body,
    );
    let accepted = stack.state.invites.accept(&hash_token(&token), T).await.expect("accept at T");
    assert_eq!(accepted.accepted_at, Some(T), "accepted_at is the injected now, not SQL now()");

    // A second invite, read after its TTL: gone from the listing, accept rejected.
    let second = format!("clock-b-{stamp}@example.com");
    let res = test::call_service(&app, invite(second.clone())).await;
    assert_eq!(res.status().as_u16(), 200, "create second invite at T");
    let mail = stack.fake_email.drain();
    let token = extract_token_from_link(
        &mail.iter().find(|m| m.to_addr == second).expect("invite mail").text_body,
    );
    let late = T + INVITE_TTL + Duration::seconds(1);
    let pending = stack.state.invites.list_pending_for_family(family_id, late).await.expect("list");
    assert!(pending.iter().all(|i| i.email != second), "expired invite is not pending");
    assert!(
        stack
            .state
            .invites
            .find_pending_by_email(family_id, &second, late)
            .await
            .expect("by email")
            .is_none(),
        "expired invite is not found by email"
    );

    stack.clock.set(late);
    let (invitee_access, _r) = sign_in(&stack, &app, &second).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/invites/accept")
        .cookie(Cookie::new("access", invitee_access))
        .set_json(serde_json::json!({ "token": token }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 410, "accept after TTL (by the injected clock) is rejected");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn magic_link_flow_follows_injected_clock() {
    let stack = ephemeral_stack().await;
    stack.clock.set(T);
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let email = format!("clock-ml-{}@example.com", rand::random::<u32>());

    // Requested and consumed at T over HTTP → OK.
    drop(sign_in(&stack, &app, &email).await);
    let user = stack.state.users.find_by_email(&email).await.expect("lookup").expect("user");

    // Repo level: consume at T stamps consumed_at = T.
    let (_token, hash) = generate_opaque_token();
    stack
        .state
        .magic_links
        .create(Some(user.id), &email, &hash, MagicLinkPurpose::Login, T + MAGIC_TTL)
        .await
        .expect("create");
    let rec = stack.state.magic_links.consume(&hash, T).await.expect("consume at T");
    assert_eq!(rec.consumed_at, Some(T), "consumed_at is the injected now, not SQL now()");

    // A link requested at T and consumed after its TTL is rejected.
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/magic-link")
        .set_json(serde_json::json!({ "email": email }))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 200, "request link at T");
    let token = extract_token_from_link(&stack.fake_email.drain().last().expect("mail").text_body);
    stack.clock.set(T + MAGIC_TTL + Duration::seconds(1));
    let req = test::TestRequest::post()
        .uri("/api/v1/auth/consume")
        .set_json(serde_json::json!({ "token": token }))
        .to_request();
    let res = common::try_call(&app, req).await;
    assert_ne!(res.status().as_u16(), 200, "consume after TTL (by the injected clock) is rejected");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refresh_token_revocation_uses_clock() {
    let stack = ephemeral_stack().await;
    stack.clock.set(T);
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let email = format!("clock-rt-{}@example.com", rand::random::<u32>());
    let (_access, refresh) = sign_in(&stack, &app, &email).await;
    let hash = hash_token(&refresh);

    stack.state.refresh_tokens.revoke_by_hash(&hash, T).await.expect("revoke at T");
    let rec = stack.state.refresh_tokens.find_any_by_hash(&hash).await.expect("find").expect("row");
    assert_eq!(rec.revoked_at, Some(T), "revoked_at is the injected now, not SQL now()");
}
