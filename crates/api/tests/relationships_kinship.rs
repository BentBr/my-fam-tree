//! Integration coverage for `GET /api/v1/relationships/kinship`.
//!
//! Drives the route + `services::relationships_kinship::build_kinship` end to
//! end: blood, in-law and partner descriptors incl. `path`, the unrelated
//! case, and every failure status the handler documents.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::shadow_unrelated,
    reason = "test code: setup failures abort the test with context; indexes into known fixture/response shapes; test steps rebind the same response/body names"
)]
#![expect(
    clippy::future_not_send,
    reason = "actix test service futures are !Send; tests run them on the current thread"
)]

mod common;

use actix_web::cookie::Cookie;
use actix_web::test;
use common::{create_family, ephemeral_stack, sign_in};
use my_fam_tree_api::build_app;
use serde_json::{Value, json};

/// Signed-in caller + active family for one request.
struct Session {
    access: String,
    family_id: String,
}

async fn post<S, B>(app: &S, session: &Session, uri: &str, payload: Value) -> Value
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let req = test::TestRequest::post()
        .uri(uri)
        .cookie(Cookie::new("access", session.access.clone()))
        .insert_header(("X-Family-Id", session.family_id.clone()))
        .set_json(payload)
        .to_request();
    let res = test::call_service(app, req).await;
    assert_eq!(res.status(), 200, "POST {uri} should succeed");
    test::read_body_json(res).await
}

async fn person<S, B>(app: &S, session: &Session, given_name: &str, gender: &str) -> String
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let body = post(
        app,
        session,
        "/api/v1/persons",
        json!({ "given_name": given_name, "gender": gender }),
    )
    .await;
    body["data"]["id"].as_str().expect("person id").to_owned()
}

/// `GET /relationships/kinship?a=..&b=..` → `(status, body)`.
async fn kinship<S, B>(app: &S, session: &Session, a: &str, b: &str) -> (u16, Value)
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/relationships/kinship?a={a}&b={b}"))
        .cookie(Cookie::new("access", session.access.clone()))
        .insert_header(("X-Family-Id", session.family_id.clone()))
        .to_request();
    let res = test::call_service(app, req).await;
    let status = res.status().as_u16();
    (status, test::read_body_json(res).await)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn kinship_reports_blood_in_law_partner_and_unrelated() {
    // Grandma -> (Dad, Aunt); Dad -> Kid; Aunt married to Uncle; Stranger.
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _refresh) = sign_in(&stack, &app, "kinship@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Kin").await;
    let session = Session { access, family_id };

    let grandma = person(&app, &session, "Grandma", "female").await;
    let dad = person(&app, &session, "Dad", "male").await;
    let aunt = person(&app, &session, "Aunt", "female").await;
    let kid = person(&app, &session, "Kid", "").await;
    let uncle = person(&app, &session, "Uncle", "male").await;
    let stranger = person(&app, &session, "Stranger", "").await;
    for (child, parent) in [(&dad, &grandma), (&aunt, &grandma), (&kid, &dad)] {
        let link = json!({ "child_id": child, "parent_id": parent, "kind": "biological" });
        post(&app, &session, "/api/v1/parent-links", link).await;
    }
    let marriage = json!({ "partner_a_id": aunt, "partner_b_id": uncle, "kind": "marriage" });
    let marriage_id =
        post(&app, &session, "/api/v1/partnerships", marriage).await["data"]["id"].clone();

    // Kid -> Aunt: blood (2,1), path Kid, Dad, Grandma, Aunt; genders echoed.
    let (status, body) = kinship(&app, &session, &kid, &aunt).await;
    assert_eq!(status, 200, "kinship should answer: {body}");
    assert_eq!(body["data"]["person_a"], json!({ "id": kid, "gender": "" }));
    assert_eq!(body["data"]["person_b"], json!({ "id": aunt, "gender": "female" }));
    assert_eq!(
        body["data"]["relations"],
        json!([{
            "up": 2, "down": 1, "half": false, "step": false,
            "in_law": null, "partnership": null, "via_partnership_id": null,
            "path": [kid, dad, grandma, aunt],
        }])
    );

    // Kid -> Uncle: the aunt's husband, an uncle by marriage.
    let (status, body) = kinship(&app, &session, &kid, &uncle).await;
    assert_eq!(status, 200, "kinship should answer: {body}");
    let relations = body["data"]["relations"].as_array().expect("relations");
    assert_eq!(relations.len(), 1, "one in-law relation expected: {body}");
    assert_eq!(relations[0]["in_law"], "via_relatives_partner");
    assert_eq!((&relations[0]["up"], &relations[0]["down"]), (&json!(2), &json!(1)));
    assert_eq!(relations[0]["path"], json!([kid, dad, grandma, aunt, uncle]));
    assert_eq!(relations[0]["via_partnership_id"], marriage_id);

    // Aunt -> Uncle: direct partners.
    let (_, body) = kinship(&app, &session, &aunt, &uncle).await;
    assert_eq!(
        body["data"]["relations"][0]["partnership"],
        json!({ "kind": "marriage", "ended_on": null, "end_reason": null })
    );
    assert_eq!(body["data"]["relations"][0]["via_partnership_id"], marriage_id);

    // Unrelated is a normal 200 with an empty list.
    let (status, body) = kinship(&app, &session, &kid, &stranger).await;
    assert_eq!(status, 200, "unrelated is not an error: {body}");
    assert_eq!(body["data"]["relations"], json!([]));

    // The tree route on the parent path still answers (no route shadowing).
    let req = test::TestRequest::get()
        .uri("/api/v1/relationships")
        .cookie(Cookie::new("access", session.access.clone()))
        .insert_header(("X-Family-Id", session.family_id.clone()))
        .to_request();
    assert_eq!(
        test::call_service(&app, req).await.status(),
        200,
        "tree route must not be shadowed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn kinship_rejects_bad_input() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _refresh) = sign_in(&stack, &app, "kinship-errors@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Mine").await;
    let mine = Session { access, family_id };
    let me = person(&app, &mine, "Me", "").await;
    let sibling = person(&app, &mine, "Sibling", "").await;

    // Same person twice -> 422 with the stable validation code.
    let (status, body) = kinship(&app, &mine, &me, &me).await;
    assert_eq!(status, 422, "a == b must be rejected: {body}");
    assert_eq!(body["fields"][0]["code"], "validation.must_differ");
    assert_eq!(body["fields"][0]["path"], "/b");

    // Not a UUID -> 400.
    let (status, body) = kinship(&app, &mine, &me, "nope").await;
    assert_eq!(status, 400, "malformed id must be rejected: {body}");

    // A person of another family is indistinguishable from a missing one.
    let (access, family_id) = create_family(&app, &mine.access, "Other").await;
    let other = Session { access, family_id };
    let foreign = person(&app, &other, "Foreign", "").await;
    for (a, b) in [(&me, &foreign), (&foreign, &me)] {
        let (status, body) = kinship(&app, &mine, a, b).await;
        assert_eq!(status, 404, "cross-family lookups must 404: {body}");
        assert_eq!(body["code"], "person_not_found");
    }
    let (status, _) = kinship(&app, &mine, &me, &sibling).await;
    assert_eq!(status, 200, "same-family pair still answers");

    // No session -> 401.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/relationships/kinship?a={me}&b={sibling}"))
        .to_request();
    let res = common::try_call(&app, req).await;
    assert_eq!(res.status(), 401, "unauthenticated must be rejected");
}
