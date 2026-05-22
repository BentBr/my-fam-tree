//! Integration coverage for `/api/v1/parent-links` — the directed
//! parent→child edge endpoints. Exercises self-parent rejection, cycle
//! detection, validation, role gating and the DELETE 404 arm.
//!
//! Transitively covers `crates/persistence/src/parent_links.rs` against a
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
use my_family_api::build_app;
use my_family_domain::{FamilyId, Role};
use uuid::Uuid;

/// Create a person under the active family and return its id as a string.
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
async fn parent_links_happy_path_and_delete_round_trip() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "pl-owner@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Edges").await;
    let child = create_person(&app, &access, &family_id, "Child").await;
    let parent = create_person(&app, &access, &family_id, "Parent").await;

    // POST happy.
    let req = test::TestRequest::post()
        .uri("/api/v1/parent-links")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "child_id": child,
            "parent_id": parent,
            "kind": "biological",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert!(body["data"].is_null());

    // DELETE happy.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/parent-links/{child}/{parent}"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);

    // Repeated DELETE -> 404 (the repo's NotFound arm maps to PersonNotFound).
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/parent-links/{child}/{parent}"))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 404);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "person_not_found");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parent_links_rejects_self_parent_and_cycles() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "pl-cycle@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Cycles").await;
    let a = create_person(&app, &access, &family_id, "A").await;
    let b = create_person(&app, &access, &family_id, "B").await;

    // Self-parent -> 409 relationship_cycle.
    let req = test::TestRequest::post()
        .uri("/api/v1/parent-links")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "child_id": a,
            "parent_id": a,
            "kind": "biological",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 409);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "relationship_cycle");

    // Insert A->B (A is child, B is parent).
    let req = test::TestRequest::post()
        .uri("/api/v1/parent-links")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "child_id": a,
            "parent_id": b,
            "kind": "biological",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);

    // Now B->A would close the cycle -> 409.
    let req = test::TestRequest::post()
        .uri("/api/v1/parent-links")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "child_id": b,
            "parent_id": a,
            "kind": "biological",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 409);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "relationship_cycle");

    // Bad kind -> 422 validation.value_required on /kind.
    let req = test::TestRequest::post()
        .uri("/api/v1/parent-links")
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id))
        .set_json(serde_json::json!({
            "child_id": Uuid::new_v4(),
            "parent_id": Uuid::new_v4(),
            "kind": "totally-bogus",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "validation_failed");
    assert_eq!(body["fields"][0]["path"], "/kind");
    assert_eq!(body["fields"][0]["code"], "validation.value_required");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parent_links_user_role_cannot_create_or_delete() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let (owner_access, _r) = sign_in(&stack, &app, "pl-owner-role@example.com").await;
    let (owner_access, family_id) = create_family(&app, &owner_access, "Roles").await;
    let _a = create_person(&app, &owner_access, &family_id, "A").await;
    let _b = create_person(&app, &owner_access, &family_id, "B").await;

    // Seed a regular user.
    let regular_email = "pl-regular@example.com";
    let _ = sign_in(&stack, &app, regular_email).await;
    let user =
        stack.state.users.find_by_email(regular_email).await.expect("lookup").expect("user exists");
    let fam_uuid = Uuid::parse_str(&family_id).expect("uuid");
    stack
        .state
        .memberships
        .insert(FamilyId::from_uuid(fam_uuid), user.id, Role::User)
        .await
        .expect("membership insert");
    let (user_access, _r) = sign_in(&stack, &app, regular_email).await;

    // POST as user -> 403.
    let req = test::TestRequest::post()
        .uri("/api/v1/parent-links")
        .cookie(Cookie::new("access", user_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "child_id": Uuid::new_v4(),
            "parent_id": Uuid::new_v4(),
            "kind": "biological",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "family_insufficient_role");

    // DELETE as user -> 403.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/parent-links/{}/{}", Uuid::new_v4(), Uuid::new_v4()))
        .cookie(Cookie::new("access", user_access))
        .insert_header(("X-Family-Id", family_id))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "family_insufficient_role");
}
