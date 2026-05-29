//! Integration coverage for `/api/v1/partnerships` — create, partial update
//! and delete of pair-canonicalized partnership rows. Exercises happy paths,
//! validation, the duplicate-open conflict, and the role-gating arms.
//!
//! Transitively covers `crates/persistence/src/partnerships.rs` against a
//! real Postgres via testcontainers.

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
use my_fam_tree_domain::{FamilyId, Role};
use uuid::Uuid;

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn partnerships_full_crud_and_duplicate_open_arm() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "pp-owner@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Partnerships").await;
    let alice = create_person(&app, &access, &family_id, "Alice").await;
    let bob = create_person(&app, &access, &family_id, "Bob").await;

    // POST happy: marriage between Alice and Bob.
    let req = test::TestRequest::post()
        .uri("/api/v1/partnerships")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "partner_a_id": alice,
            "partner_b_id": bob,
            "kind": "marriage",
            "note": "hello",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let partnership_id = body["data"]["id"].as_str().expect("partnership id").to_string();
    // The repo canonicalises the pair so partner_a_id < partner_b_id.
    let a_uuid = body["data"]["partner_a_id"].as_str().unwrap().to_string();
    let b_uuid = body["data"]["partner_b_id"].as_str().unwrap().to_string();
    assert!(a_uuid < b_uuid, "canonical order should be partner_a < partner_b");
    assert_eq!(body["data"]["kind"], "marriage");
    assert_eq!(body["data"]["note"], "hello");

    // Duplicate marriage on the same pair while the first is still open -> 409.
    let req = test::TestRequest::post()
        .uri("/api/v1/partnerships")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            // Use the reversed order to also prove canonicalisation kicks in.
            "partner_a_id": bob,
            "partner_b_id": alice,
            "kind": "marriage",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 409);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "partnership_duplicate");

    // PATCH happy: change the note.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/partnerships/{partnership_id}"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({ "note": "renamed" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["note"], "renamed");

    // Empty PATCH -> 422.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/partnerships/{partnership_id}"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({}))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.value_required");

    // DELETE happy.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/partnerships/{partnership_id}"))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert!(body["data"].is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn partnerships_validation_for_kind_and_end_reason() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "pp-val@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Validation").await;
    let p1 = create_person(&app, &access, &family_id, "P1").await;
    let p2 = create_person(&app, &access, &family_id, "P2").await;

    // Bad kind.
    let req = test::TestRequest::post()
        .uri("/api/v1/partnerships")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "partner_a_id": p1,
            "partner_b_id": p2,
            "kind": "open-relationship",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["path"], "/kind");

    // Bad end_reason.
    let req = test::TestRequest::post()
        .uri("/api/v1/partnerships")
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id))
        .set_json(serde_json::json!({
            "partner_a_id": p1,
            "partner_b_id": p2,
            "kind": "marriage",
            "end_reason": "stardust",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["path"], "/end_reason");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn partnerships_user_role_blocked_on_all_mutations() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (owner_access, _r) = sign_in(&stack, &app, "pp-owner-role@example.com").await;
    let (owner_access, family_id) = create_family(&app, &owner_access, "RolesPP").await;
    let p1 = create_person(&app, &owner_access, &family_id, "P1").await;
    let p2 = create_person(&app, &owner_access, &family_id, "P2").await;

    // Create one as owner so the user can attempt to mutate it.
    let req = test::TestRequest::post()
        .uri("/api/v1/partnerships")
        .cookie(Cookie::new("access", owner_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "partner_a_id": p1,
            "partner_b_id": p2,
            "kind": "civil_union",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let partnership_id = body["data"]["id"].as_str().unwrap().to_string();

    // Seed a regular user.
    let regular = "pp-regular@example.com";
    let _ = sign_in(&stack, &app, regular).await;
    let user = stack.state.users.find_by_email(regular).await.expect("lookup").expect("user");
    let fam_uuid = Uuid::parse_str(&family_id).expect("uuid");
    stack
        .state
        .memberships
        .insert(FamilyId::from_uuid(fam_uuid), user.id, Role::User)
        .await
        .expect("membership");
    let (user_access, _r) = sign_in(&stack, &app, regular).await;

    // POST -> 403.
    let req = test::TestRequest::post()
        .uri("/api/v1/partnerships")
        .cookie(Cookie::new("access", user_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "partner_a_id": p1,
            "partner_b_id": p2,
            "kind": "partnership",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);

    // PATCH -> 403.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/partnerships/{partnership_id}"))
        .cookie(Cookie::new("access", user_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({ "note": "nope" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);

    // DELETE -> 403.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/partnerships/{partnership_id}"))
        .cookie(Cookie::new("access", user_access))
        .insert_header(("X-Family-Id", family_id))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);
}
