//! Integration coverage for `POST/DELETE /api/v1/persons/{id}/photo`.
//!
//! Exercises the upload pipeline end-to-end against a real
//! `LocalObjectStore` (per-test tempdir, see `common::ephemeral_stack`):
//!
//! - Tiny in-memory PNG → 200, response carries `photo_key` + `photo_url`.
//! - Subsequent `GET /persons/{id}` surfaces `photo_url` on `PersonView`.
//! - Replacing the photo deletes the previous key from the store (uses the
//!   CTE-backed `set_photo_key` repo path, so the swap is atomic).
//! - DELETE clears `photo_url` and removes the underlying object.
//! - Rejection paths: non-image bytes → 422 `image.invalid`; PNG bytes with
//!   a `.jpg` filename → 422 (extension mismatch).
//! - Cross-family IDOR guard: an admin of F1 cannot set a photo on a person
//!   that lives in F2 (the audit High shape, mirrored on the photo route).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::future_not_send,
    clippy::shadow_unrelated,
    clippy::shadow_reuse,
    clippy::shadow_same,
    clippy::similar_names,
    clippy::case_sensitive_file_extension_comparisons,
    reason = "test code: testcontainers + assertion helpers may panic and aren't Send-bounded; shadowing matches the existing flow tests; access_a/access_b are the natural pair names for the cross-family test"
)]

mod common;

use actix_web::cookie::Cookie;
use actix_web::test;
use common::{create_family, ephemeral_stack, sign_in};
use image::ImageFormat;
use my_fam_tree_api::build_app;

/// Returns a fresh PNG (raw bytes) — minted via the image crate so the
/// fixture isn't checked in as a binary. 4×4 pixels keeps the encode fast.
fn tiny_png(red: u8, green: u8, blue: u8) -> Vec<u8> {
    let img = image::RgbImage::from_fn(4, 4, |_, _| image::Rgb([red, green, blue]));
    let mut buf = Vec::new();
    image::DynamicImage::from(img)
        .write_to(&mut std::io::Cursor::new(&mut buf), ImageFormat::Png)
        .unwrap();
    buf
}

/// Build the multipart body for a `file=<bytes>` upload with the given filename.
/// Returns `(content_type_header, body_bytes)`.
fn multipart_body(filename: &str, bytes: &[u8]) -> (String, Vec<u8>) {
    // Boundary picked so it can't appear inside a PNG. The actual multipart
    // parser tolerates any token; we just need it to be a string the
    // payload itself doesn't contain.
    let boundary = "----photoflowboundary7e2";
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
async fn upload_photo_then_get_returns_presigned_url_then_delete_clears_it() {
    let stack = ephemeral_stack().await;
    let store = stack.state.object_store.clone();
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "photo-owner@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Photos").await;
    let person_id = create_person(&app, &access, &family_id, "Karin").await;

    // ----- POST happy path -----
    let png = tiny_png(255, 0, 0);
    let (ct, body) = multipart_body("karin.png", &png);
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/persons/{person_id}/photo"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .insert_header(("Content-Type", ct))
        .set_payload(body)
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200, "upload should succeed");
    let body: serde_json::Value = test::read_body_json(res).await;
    let key1 = body["data"]["photo_key"].as_str().expect("photo_key").to_string();
    let url1 = body["data"]["photo_url"].as_str().expect("photo_url");
    assert!(key1.starts_with(&format!("persons/{person_id}/")), "key shape: got `{key1}`");
    assert!(key1.ends_with(".jpg"), "output extension is always .jpg, got `{key1}`");
    assert!(!url1.is_empty(), "presigned url should not be empty");
    // Bytes landed in the store; they're JPEG (validator re-encodes regardless
    // of input format), so the magic-byte prefix matches FF D8 FF.
    let stored = store.get(&key1).await.expect("get bytes back");
    assert_eq!(&stored[..3], &[0xFF, 0xD8, 0xFF], "stored bytes are JPEG");

    // ----- GET /persons/{id} surfaces photo_url -----
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert!(body["data"]["photo_url"].is_string(), "PersonView carries photo_url");

    // ----- Replace photo: new key minted, old object deleted -----
    let png2 = tiny_png(0, 255, 0);
    let (ct, body) = multipart_body("karin-v2.png", &png2);
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/persons/{person_id}/photo"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .insert_header(("Content-Type", ct))
        .set_payload(body)
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let key2 = body["data"]["photo_key"].as_str().expect("photo_key").to_string();
    assert_ne!(key1, key2, "replace mints a fresh suffix");
    match store.get(&key1).await {
        Err(my_fam_tree_storage::StorageError::NotFound(_)) => {} // expected
        other => panic!("previous key should be deleted, got {other:?}"),
    }

    // ----- DELETE clears the field + removes the object -----
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/persons/{person_id}/photo"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/persons/{person_id}"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    assert!(body["data"]["photo_url"].is_null(), "photo_url null after DELETE");
    match store.get(&key2).await {
        Err(my_fam_tree_storage::StorageError::NotFound(_)) => {}
        other => panic!("post-DELETE object should be gone, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upload_rejects_non_image_bytes_with_image_invalid() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "photo-bad@example.com").await;
    let (access, family_id) = create_family(&app, &access, "BadInput").await;
    let person_id = create_person(&app, &access, &family_id, "Karin").await;

    let (ct, body) = multipart_body("nonsense.png", b"definitely not an image");
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/persons/{person_id}/photo"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id))
        .insert_header(("Content-Type", ct))
        .set_payload(body)
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "image_invalid");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upload_rejects_extension_mismatch() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "photo-ext@example.com").await;
    let (access, family_id) = create_family(&app, &access, "ExtCheck").await;
    let person_id = create_person(&app, &access, &family_id, "Karin").await;

    // PNG bytes with a .jpg filename — the magic-bytes/extension cross-check
    // must trip the 422 image.invalid path.
    let png = tiny_png(50, 50, 50);
    let (ct, body) = multipart_body("trying.jpg", &png);
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/persons/{person_id}/photo"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id))
        .insert_header(("Content-Type", ct))
        .set_payload(body)
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "image_invalid");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upload_rejects_cross_family_person_id() {
    let stack = ephemeral_stack().await;
    let store = stack.state.object_store.clone();
    let app = test::init_service(build_app(stack.state.clone(), None)).await;

    // Family A owns the person; Family B owns the attacker.
    let (access_a, _r) = sign_in(&stack, &app, "photo-fam-a@example.com").await;
    let (access_a, family_a_id) = create_family(&app, &access_a, "FamilyA").await;
    let person_a = create_person(&app, &access_a, &family_a_id, "Karin").await;

    let (access_b, _r) = sign_in(&stack, &app, "photo-fam-b@example.com").await;
    let (access_b, family_b_id) = create_family(&app, &access_b, "FamilyB").await;

    // Attacker is in family B; tries to set a photo on family A's person.
    let png = tiny_png(123, 45, 67);
    let (ct, body) = multipart_body("evil.png", &png);
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/persons/{person_a}/photo"))
        .cookie(Cookie::new("access", access_b.clone()))
        .insert_header(("X-Family-Id", family_b_id))
        .insert_header(("Content-Type", ct))
        .set_payload(body)
        .to_request();
    let res = test::call_service(&app, req).await;
    // The handler resolves the person through find_in_family(active.id, _)
    // BEFORE the bytes are written, so the 404 lands clean and no object
    // ever hits the store.
    assert_eq!(res.status(), 404, "cross-family photo set must 404");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "person_not_found");

    // Sanity: the store has no keys under family A's person.
    match store.get(&format!("persons/{person_a}/anything.jpg")).await {
        Err(my_fam_tree_storage::StorageError::NotFound(_)) => {}
        other => panic!("no rogue object should exist, got {other:?}"),
    }
}
