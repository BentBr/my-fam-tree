//! Authorization matrix for the three roles (owner / admin / user).
//!
//! The FE gates content cosmetically; the API is the real gate. This proves
//! each role's access against a freshly-seeded three-role family: non-owners
//! are denied owner-only endpoints, non-admins are denied admin-only endpoints
//! (403 `family.insufficient_role`), and every role keeps its read access.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::future_not_send,
    clippy::shadow_unrelated,
    clippy::shadow_reuse,
    clippy::shadow_same,
    reason = "test code: testcontainers + assertion helpers may panic and aren't Send-bounded; shadowing matches the existing integration-test convention"
)]

mod common;

use actix_web::cookie::Cookie;
use actix_web::test;
use common::{ephemeral_stack, fresh_access, seed_three_role_family};
use my_family_api::build_app;

/// Issue a request as `access` against `family_id` and return the HTTP status.
#[allow(clippy::future_not_send)]
async fn status<S, B>(
    app: &S,
    access: &str,
    family_id: &str,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> u16
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let mut req = match method {
        "POST" => test::TestRequest::post(),
        "PATCH" => test::TestRequest::patch(),
        "DELETE" => test::TestRequest::delete(),
        _ => test::TestRequest::get(),
    }
    .uri(uri)
    .cookie(Cookie::new("access", access.to_string()))
    .insert_header(("X-Family-Id", family_id.to_string()));
    if let Some(b) = body {
        req = req.set_json(b);
    }
    test::call_service(app, req.to_request()).await.status().as_u16()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn role_matrix_denies_and_allows_the_right_endpoints() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let stamp = u128::from(rand::random::<u32>());
    let fam = seed_three_role_family(&stack, &app, stamp).await;
    let fid = fam.family_id.into_uuid().to_string();

    let user = fresh_access(&stack, &app, &fam.user_email).await;
    let admin = fresh_access(&stack, &app, &fam.admin_email).await;
    let owner = fresh_access(&stack, &app, &fam.owner_email).await;

    let person = || Some(serde_json::json!({ "given_name": "A", "family_name": "B" }));

    // --- user: denied every admin/owner gate, allowed its reads ---
    assert_eq!(status(&app, &user, &fid, "POST", "/api/v1/persons", person()).await, 403, "user create person");
    assert_eq!(status(&app, &user, &fid, "DELETE", &format!("/api/v1/families/{fid}"), None).await, 403, "user delete family");
    assert_eq!(status(&app, &user, &fid, "GET", &format!("/api/v1/families/{fid}/audit"), None).await, 403, "user audit");
    assert_eq!(status(&app, &user, &fid, "GET", &format!("/api/v1/families/{fid}/members"), None).await, 403, "user members");
    assert_eq!(status(&app, &user, &fid, "GET", "/api/v1/persons", None).await, 200, "user list persons");
    assert_eq!(status(&app, &user, &fid, "GET", "/api/v1/families/me", None).await, 200, "user families/me");
    assert_eq!(status(&app, &user, &fid, "GET", "/api/v1/reminder-preferences", None).await, 200, "user reminder prefs");

    // The denial code is specifically the role gate (not a generic 403).
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", user.clone()))
        .insert_header(("X-Family-Id", fid.clone()))
        .set_json(serde_json::json!({ "given_name": "A", "family_name": "B" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    let denied: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(denied["code"], "family.insufficient_role", "user create person → role gate");

    // --- admin: admin gates pass, owner gate denied ---
    assert_eq!(status(&app, &admin, &fid, "POST", "/api/v1/persons", person()).await, 200, "admin create person");
    assert_eq!(status(&app, &admin, &fid, "GET", &format!("/api/v1/families/{fid}/audit"), None).await, 200, "admin audit");
    assert_eq!(status(&app, &admin, &fid, "GET", &format!("/api/v1/families/{fid}/members"), None).await, 200, "admin members");
    // Bodyless DELETE reaches require_role(Owner) directly (no extractor to
    // 400 first), cleanly proving the owner-only gate denies an admin.
    assert_eq!(status(&app, &admin, &fid, "DELETE", &format!("/api/v1/families/{fid}"), None).await, 403, "admin delete family (owner-only)");

    // --- owner: admin-or-higher gates pass ---
    assert_eq!(status(&app, &owner, &fid, "POST", "/api/v1/persons", person()).await, 200, "owner create person");
    assert_eq!(status(&app, &owner, &fid, "GET", &format!("/api/v1/families/{fid}/audit"), None).await, 200, "owner audit");
}
