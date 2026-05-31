//! Integration coverage for `POST/DELETE /api/v1/users/me/avatar`.
//!
//! Mirrors `person_photo_flow.rs` but on the calling user's avatar. The
//! upload pipeline broadcasts the new photo to every `persons` row where
//! `linked_user_id = self`, so the test asserts both the avatar fields
//! AND the linked-person photo propagation; clearing reverses both.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::future_not_send,
    clippy::shadow_unrelated,
    clippy::shadow_reuse,
    clippy::shadow_same,
    reason = "test code: testcontainers + assertion helpers may panic and aren't Send-bounded; shadowing matches the existing flow tests"
)]

mod common;

use actix_web::cookie::Cookie;
use actix_web::test;
use common::{create_family, ephemeral_stack, sign_in};
use image::ImageFormat;
use my_fam_tree_api::build_app;
use my_fam_tree_domain::{FamilyId, PersonId};

/// Tiny in-memory PNG — same trick as `person_photo_flow` so the fixture
/// isn't a checked-in binary.
fn tiny_png(r: u8, g: u8, b: u8) -> Vec<u8> {
    let img = image::RgbImage::from_fn(4, 4, |_, _| image::Rgb([r, g, b]));
    let mut buf = Vec::new();
    image::DynamicImage::from(img)
        .write_to(&mut std::io::Cursor::new(&mut buf), ImageFormat::Png)
        .unwrap();
    buf
}

fn multipart_body(filename: &str, bytes: &[u8]) -> (String, Vec<u8>) {
    let boundary = "----avatarflowboundary42";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: image/png\r\n\r\n");
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}

// Multi-stage flow (upload + propagate + replace + delete + re-check)
// naturally runs long; splitting it into helpers would scatter the
// assertions and obscure the round-trip. Localised allow.
#[allow(clippy::too_many_lines)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upload_avatar_sets_url_propagates_to_linked_persons_and_clear_undoes_it() {
    let stack = ephemeral_stack().await;
    let store = stack.state.object_store.clone();
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let owner_email = "avatar-owner@example.com";
    let (access, _r) = sign_in(&stack, &app, owner_email).await;
    let (access, family_id_str) = create_family(&app, &access, "AvatarFam").await;
    let family_uuid: uuid::Uuid = family_id_str.parse().unwrap();
    let family_id = FamilyId::from_uuid(family_uuid);

    // Find the caller's user id + wire up a linked person via the repo so
    // the upload's propagation step has a target row to write through to.
    // (POST /persons doesn't accept `linked_user_id` for other users since
    // the consent gate landed — the test bypass is the repo, matching
    // the pattern used by person_contacts_flow.)
    let owner_user = stack
        .state
        .users
        .find_by_email(owner_email)
        .await
        .expect("user lookup")
        .expect("owner user");
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id_str.clone()))
        .set_json(serde_json::json!({ "given_name": "Self" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let person_id_str = body["data"]["id"].as_str().unwrap().to_string();
    let person_id =
        PersonId::from_uuid(uuid::Uuid::parse_str(&person_id_str).expect("person uuid"));
    stack
        .state
        .persons
        .set_linked_user_id(family_id, person_id, Some(owner_user.id))
        .await
        .expect("repo set_linked_user_id fixture");

    // ----- POST happy path: upload PNG, get presigned URL back -----
    let png = tiny_png(50, 100, 150);
    let (ct, body) = multipart_body("me.png", &png);
    let req = test::TestRequest::post()
        .uri("/api/v1/users/me/avatar")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("content-type", ct))
        .set_payload(body)
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200, "upload should succeed");
    let body: serde_json::Value = test::read_body_json(res).await;
    let avatar_key = body["data"]["avatar_key"].as_str().expect("avatar_key").to_string();
    let avatar_url = body["data"]["avatar_url"].as_str().expect("avatar_url");
    assert!(avatar_url.starts_with("http"), "avatar_url is a presigned URL");
    // The key sits under `users/{user_id}/`.
    assert!(
        avatar_key.starts_with(&format!("users/{}/", owner_user.id.into_uuid())),
        "avatar key scoped to the user's own subtree, got `{avatar_key}`",
    );
    // Object actually landed in the store (no `exists` on the trait;
    // `get` succeeding is the equivalent assertion).
    assert!(store.get(&avatar_key).await.is_ok(), "uploaded avatar is in the object store");

    // ----- GET /users/me surfaces the avatar_url -----
    let req = test::TestRequest::get()
        .uri("/api/v1/users/me")
        .cookie(Cookie::new("access", access.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let me_url = body["data"]["avatar_url"].as_str().expect("avatar_url on /users/me");
    assert!(me_url.starts_with("http"));

    // ----- Linked person picks up the same photo_key via the broadcast -----
    let linked = stack
        .state
        .persons
        .find_in_family(family_id, person_id)
        .await
        .expect("find_in_family")
        .expect("linked person");
    assert_eq!(
        linked.photo_key.as_deref(),
        Some(avatar_key.as_str()),
        "avatar propagates to persons where linked_user_id = self",
    );

    // ----- Replace the avatar: previous key gets cleaned up -----
    let png2 = tiny_png(200, 200, 200);
    let (ct, body) = multipart_body("me-new.png", &png2);
    let req = test::TestRequest::post()
        .uri("/api/v1/users/me/avatar")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("content-type", ct))
        .set_payload(body)
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let new_key = body["data"]["avatar_key"].as_str().unwrap().to_string();
    assert_ne!(new_key, avatar_key, "second upload mints a fresh key");
    // Old key cleaned up best-effort; new key stored.
    assert!(store.get(&avatar_key).await.is_err(), "previous key deleted");
    assert!(store.get(&new_key).await.is_ok(), "new key stored");

    // ----- DELETE clears the avatar + propagates the null to linked persons -----
    let req = test::TestRequest::delete()
        .uri("/api/v1/users/me/avatar")
        .cookie(Cookie::new("access", access.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert!(body["data"].is_null(), "DELETE returns null data per the spec");
    assert!(store.get(&new_key).await.is_err(), "store key removed");

    let req = test::TestRequest::get()
        .uri("/api/v1/users/me")
        .cookie(Cookie::new("access", access.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    assert!(body["data"]["avatar_url"].is_null(), "avatar_url null after delete");
    let linked = stack.state.persons.find_in_family(family_id, person_id).await.unwrap().unwrap();
    assert!(linked.photo_key.is_none(), "linked person's photo_key cleared too");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upload_rejects_non_image_bytes_with_422_image_invalid() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "garbage-uploader@example.com").await;

    let (ct, body) = multipart_body("not-an-image.png", b"this is plain text not a PNG");
    let req = test::TestRequest::post()
        .uri("/api/v1/users/me/avatar")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("content-type", ct))
        .set_payload(body)
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "image_invalid");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upload_without_session_returns_401() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    let (ct, body) = multipart_body("me.png", &tiny_png(0, 0, 0));
    let req = test::TestRequest::post()
        .uri("/api/v1/users/me/avatar")
        .insert_header(("content-type", ct))
        .set_payload(body)
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 401);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_when_no_avatar_set_is_a_graceful_noop() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "no-avatar@example.com").await;

    let req = test::TestRequest::delete()
        .uri("/api/v1/users/me/avatar")
        .cookie(Cookie::new("access", access))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200, "delete without an existing avatar succeeds idempotently");
}
