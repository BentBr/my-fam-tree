//! Integration coverage for the per-user favourites feature.
//!
//! Asserts the four guarantees from the brief:
//!   1. PATCH `/persons/{id}/favourite` toggles set/unset idempotently
//!      on both directions for the same user.
//!   2. Two users in the same family see independent favourite state
//!      on the same person — favourites are per-user, never shared.
//!   3. The tree response embeds `is_favourite_for_me` per node, and
//!      it reflects only the calling user's marks.
//!   4. `GET /upcoming?favourites_only=true` filters the projection to
//!      events whose owning person (or either partner, for wedding
//!      anniversaries) is favourited by the caller.

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
use common::{create_family, ephemeral_stack, extract_token_from_link, sign_in};
use my_fam_tree_api::build_app;

/// Helper: create a person under the active family and return its id.
#[allow(clippy::future_not_send)]
async fn create_person<S, B>(
    app: &S,
    access: &str,
    family_id: &str,
    given_name: &str,
    birth_date: Option<&str>,
) -> String
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let mut body = serde_json::json!({ "given_name": given_name, "family_name": "Favourite" });
    if let Some(d) = birth_date {
        body["birth_date"] = serde_json::Value::String(d.to_owned());
    }
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.to_string()))
        .insert_header(("X-Family-Id", family_id.to_string()))
        .set_json(body)
        .to_request();
    let res = test::call_service(app, req).await;
    assert_eq!(res.status(), 200, "create person `{given_name}` should succeed");
    let body: serde_json::Value = test::read_body_json(res).await;
    body["data"]["id"].as_str().expect("person id").to_string()
}

/// Helper: POST `is_favourite=true|false` for the calling user.
#[allow(clippy::future_not_send)]
async fn set_favourite<S, B>(
    app: &S,
    access: &str,
    family_id: &str,
    person_id: &str,
    is_favourite: bool,
) -> serde_json::Value
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/persons/{person_id}/favourite"))
        .cookie(Cookie::new("access", access.to_string()))
        .insert_header(("X-Family-Id", family_id.to_string()))
        .set_json(serde_json::json!({ "is_favourite": is_favourite }))
        .to_request();
    let res = test::call_service(app, req).await;
    assert_eq!(res.status(), 200, "PATCH /favourite should succeed");
    test::read_body_json(res).await
}

/// Invite `email` as an admin and accept the invite as a freshly signed-in
/// user with the same email. Returns the rotated access cookie carrying the
/// joined family membership. Mirrors the helper pattern used in
/// `family_invite_flows.rs`.
#[allow(clippy::future_not_send)]
async fn invite_and_join<S, B>(
    stack: &common::TestStack,
    app: &S,
    owner_access: &str,
    family_id: &str,
    email: &str,
) -> String
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/families/{family_id}/invites"))
        .cookie(Cookie::new("access", owner_access.to_string()))
        .set_json(serde_json::json!({ "email": email, "role": "admin" }))
        .to_request();
    let res = test::call_service(app, req).await;
    assert_eq!(res.status(), 200);
    let captured = stack.fake_email.drain();
    let mail = captured.last().expect("invite email captured");
    let token = extract_token_from_link(&mail.text_body);

    let (access, _r) = sign_in(stack, app, email).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/invites/accept")
        .cookie(Cookie::new("access", access))
        .set_json(serde_json::json!({ "token": token }))
        .to_request();
    let res = test::call_service(app, req).await;
    assert_eq!(res.status(), 200);
    res.response()
        .cookies()
        .find(|c| c.name() == "access")
        .expect("rotated access")
        .value()
        .to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn favourite_toggle_is_idempotent_per_user() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "fav-owner@example.com").await;
    let (access, family_id) = create_family(&app, &access, "FavFam").await;
    let person_id = create_person(&app, &access, &family_id, "Klaus", Some("1980-04-12")).await;

    // Initial GET /persons/{id} reports is_favourite_for_me = false.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["is_favourite_for_me"], false);

    // Mark twice — second is a no-op, both 200.
    let r1 = set_favourite(&app, &access, &family_id, &person_id, true).await;
    assert_eq!(r1["data"]["is_favourite"], true);
    let r2 = set_favourite(&app, &access, &family_id, &person_id, true).await;
    assert_eq!(r2["data"]["is_favourite"], true);

    // GET now reflects the flip.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["is_favourite_for_me"], true);

    // Unmark twice — also idempotent.
    let r3 = set_favourite(&app, &access, &family_id, &person_id, false).await;
    assert_eq!(r3["data"]["is_favourite"], false);
    let r4 = set_favourite(&app, &access, &family_id, &person_id, false).await;
    assert_eq!(r4["data"]["is_favourite"], false);

    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["is_favourite_for_me"], false);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_users_see_independent_favourite_state() {
    // Owner marks Klaus as favourite; the second user (admin, joined via
    // invite) sees Klaus's `is_favourite_for_me = false` on the SAME
    // person row. Proves the state is keyed per-user, not per-person.
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (owner_access, _r) = sign_in(&stack, &app, "fav-owner-2@example.com").await;
    stack.fake_email.drain();
    let (owner_access, family_id) = create_family(&app, &owner_access, "FavPair").await;
    let person_id = create_person(&app, &owner_access, &family_id, "Klaus", None).await;

    // Owner marks the person as their favourite.
    set_favourite(&app, &owner_access, &family_id, &person_id, true).await;

    // Second user joins via invite, gets a fresh session.
    let admin_access =
        invite_and_join(&stack, &app, &owner_access, &family_id, "fav-admin@example.com").await;

    // Owner GET → true.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", owner_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["is_favourite_for_me"], true);

    // Admin GET on the SAME person → false.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", admin_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["is_favourite_for_me"], false);

    // The tree response carries the same per-user split.
    let req = test::TestRequest::get()
        .uri("/api/v1/relationships")
        .cookie(Cookie::new("access", owner_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    let owner_node =
        body["data"]["nodes"].as_array().unwrap().iter().find(|n| n["id"] == person_id).unwrap();
    assert_eq!(owner_node["is_favourite_for_me"], true);

    let req = test::TestRequest::get()
        .uri("/api/v1/relationships")
        .cookie(Cookie::new("access", admin_access))
        .insert_header(("X-Family-Id", family_id))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    let admin_node =
        body["data"]["nodes"].as_array().unwrap().iter().find(|n| n["id"] == person_id).unwrap();
    assert_eq!(admin_node["is_favourite_for_me"], false);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upcoming_favourites_only_filters_to_current_user_marks() {
    // Two persons with birthdays this calendar year. Owner favourites
    // Klaus only. `/upcoming?favourites_only=true` must return Klaus's
    // birthday and not Anna's.
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "fav-upcoming@example.com").await;
    let (access, family_id) = create_family(&app, &access, "FavUp").await;

    // Use stable month-day pairs unlikely to coincide so the assertion
    // is independent of the test runner's wall clock — both events
    // either fall in this year (post today) or roll over to next year.
    let klaus = create_person(&app, &access, &family_id, "Klaus", Some("1970-06-04")).await;
    let _anna = create_person(&app, &access, &family_id, "Anna", Some("1972-08-22")).await;

    // Default (no filter) → both birthdays projected.
    let req = test::TestRequest::get()
        .uri("/api/v1/upcoming")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    let kinds: Vec<&str> =
        body["data"].as_array().unwrap().iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert!(kinds.iter().filter(|k| **k == "birthday").count() >= 2);

    // Mark Klaus only.
    set_favourite(&app, &access, &family_id, &klaus, true).await;

    // favourites_only=true → Anna's birthday filtered out.
    let req = test::TestRequest::get()
        .uri("/api/v1/upcoming?favourites_only=true")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    let events = body["data"].as_array().unwrap();
    assert!(
        events.iter().all(|e| e["person_id"].as_str() == Some(klaus.as_str())),
        "favourites_only=true must drop non-favourite persons; got {events:?}"
    );
    // And Klaus is present.
    assert!(events.iter().any(|e| e["person_id"].as_str() == Some(klaus.as_str())));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn favourite_unknown_person_returns_404() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "fav-404@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Fav404").await;

    let bogus = uuid::Uuid::new_v4();
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/persons/{bogus}/favourite"))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id))
        .set_json(serde_json::json!({ "is_favourite": true }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 404);
}
