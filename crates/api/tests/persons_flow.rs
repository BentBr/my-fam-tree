//! Integration coverage for `/api/v1/persons` against ephemeral Postgres +
//! Redis containers.
//!
//! Drives full CRUD plus the role-gating, validation and pagination arms of
//! the handlers. The route file is otherwise entirely uncovered by unit
//! tests; exercising every endpoint here transitively also covers
//! `crates/persistence/src/persons.rs` against a real database.

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

fn person_create_body(given_name: &str) -> serde_json::Value {
    serde_json::json!({
        "given_name": given_name,
        "family_name": "Doe",
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn persons_full_crud_happy_path() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _refresh) = sign_in(&stack, &app, "owner-persons@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Personsville").await;

    // GET /persons — empty list, pagination block reflects zero rows.
    let req = test::TestRequest::get()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
    assert_eq!(body["meta"]["pagination"]["returned"], 0);
    assert_eq!(body["meta"]["pagination"]["limit"], 50);
    assert!(body["meta"]["pagination"]["next_cursor"].is_null());

    // POST /persons — happy create.
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(person_create_body("Anna"))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["given_name"], "Anna");
    assert_eq!(body["data"]["family_name"], "Doe");
    assert_eq!(body["data"]["family_id"], family_id);
    let person_id = body["data"]["id"].as_str().expect("person id").to_string();

    // GET /persons — list now has one row.
    let req = test::TestRequest::get()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"][0]["given_name"], "Anna");

    // GET /persons/{id} — found.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["id"], person_id);

    // PATCH /persons/{id} — happy partial update on family_name.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({ "family_name": "Renamed" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["family_name"], "Renamed");
    // given_name unchanged after partial update.
    assert_eq!(body["data"]["given_name"], "Anna");

    // DELETE /persons/{id} — happy delete, `{ "data": null }`.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert!(body["data"].is_null());

    // GET /persons/{id} — now 404.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 404);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "person_not_found");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn persons_validation_and_404_arms() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _refresh) = sign_in(&stack, &app, "validator@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Validation Fam").await;

    // POST /persons with blank given_name -> 422 value_required.
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({ "given_name": "   " }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "validation_failed");
    assert_eq!(body["fields"][0]["code"], "validation.value_required");
    assert_eq!(body["fields"][0]["path"], "/given_name");

    // GET /persons/{unknown} -> 404 person_not_found.
    let bogus = Uuid::new_v4();
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/persons/{bogus}"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 404);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "person_not_found");

    // Empty PATCH body -> 422 value_required on /.
    // First create a person we can target.
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(person_create_body("Patchee"))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let person_id = body["data"]["id"].as_str().expect("person id").to_string();

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({}))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["code"], "validation.value_required");
    assert_eq!(body["fields"][0]["path"], "/");

    // PATCH with blank given_name -> 422 value_required on /given_name.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id))
        .set_json(serde_json::json!({ "given_name": "   " }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["fields"][0]["path"], "/given_name");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn persons_role_gating_user_cannot_create_or_delete_or_edit_others() {
    // Owner creates the family + a person; a regular `user`-role member then
    // tries to delete and edit it. Both arms fail with 403 codes.
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let owner_email = "owner-role@example.com";
    let (owner_access, _r) = sign_in(&stack, &app, owner_email).await;
    let (owner_access, family_id) = create_family(&app, &owner_access, "Role Fam").await;

    // Pre-create a person as owner.
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", owner_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(person_create_body("Owned"))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let person_id = body["data"]["id"].as_str().expect("person id").to_string();

    // Sign in a second user, then directly seed a `user`-role membership in
    // the same family so the next sign_in picks it up via the JWT claims.
    let regular_email = "regular-role@example.com";
    let (_throwaway_access, _r) = sign_in(&stack, &app, regular_email).await;
    let user =
        stack.state.users.find_by_email(regular_email).await.expect("lookup").expect("user exists");
    let fam_uuid = Uuid::parse_str(&family_id).expect("uuid");
    stack
        .state
        .memberships
        .insert(FamilyId::from_uuid(fam_uuid), user.id, Role::User)
        .await
        .expect("membership insert");
    // Re-sign-in so the access token contains the new membership.
    let (user_access, _r) = sign_in(&stack, &app, regular_email).await;

    // POST /persons as user -> 403 family_insufficient_role.
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", user_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(person_create_body("NoCreate"))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "family_insufficient_role");

    // PATCH on someone else's person row -> 403 person_not_editable.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", user_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({ "family_name": "Hacked" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "person_not_editable");

    // DELETE as user -> 403 family_insufficient_role.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", user_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "family_insufficient_role");

    // Now link a person row to the regular user — they should then be able
    // to PATCH that row but still NOT someone else's.
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", owner_access))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "given_name": "Self",
            "linked_user_id": user.id.into_uuid().to_string(),
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let self_person_id = body["data"]["id"].as_str().expect("person id").to_string();
    // Belt-and-braces: confirm the repo agrees.
    let by_link = stack
        .state
        .persons
        .find_by_linked_user(FamilyId::from_uuid(fam_uuid), user.id)
        .await
        .expect("find_by_linked_user")
        .expect("linked");
    assert_eq!(by_link.id.into_uuid().to_string(), self_person_id);

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/persons/{self_person_id}"))
        .cookie(Cookie::new("access", user_access))
        .insert_header(("X-Family-Id", family_id))
        .set_json(serde_json::json!({ "nickname": "Me" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["nickname"], "Me");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn persons_cursor_pagination_walks_full_list() {
    // Insert three persons, then page through with limit=2: first page returns
    // two and a non-null `next_cursor`; second page returns one and a null
    // cursor. Verifies the `?cursor=...&limit=2` path through the list handler.
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "paginator@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Pagey").await;

    for name in ["Alpha", "Bravo", "Charlie"] {
        let req = test::TestRequest::post()
            .uri("/api/v1/persons")
            .cookie(Cookie::new("access", access.clone()))
            .insert_header(("X-Family-Id", family_id.clone()))
            .set_json(person_create_body(name))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200);
    }

    // First page.
    let req = test::TestRequest::get()
        .uri("/api/v1/persons?limit=2")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let first = body["data"].as_array().unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(body["meta"]["pagination"]["returned"], 2);
    assert_eq!(body["meta"]["pagination"]["limit"], 2);
    let next_cursor =
        body["meta"]["pagination"]["next_cursor"].as_str().expect("next cursor").to_string();

    // Second page.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/persons?limit=2&cursor={next_cursor}"))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let second = body["data"].as_array().unwrap();
    assert_eq!(second.len(), 1);
    assert_eq!(body["meta"]["pagination"]["returned"], 1);
    // Shorter than the limit -> last page, no further cursor.
    assert!(body["meta"]["pagination"]["next_cursor"].is_null());
}

// Coverage for the new contact columns + email-from-linked-user sync
// lives in `persons_contact_flow.rs` so this file stays inside the
// 500-line test-binary cap.
