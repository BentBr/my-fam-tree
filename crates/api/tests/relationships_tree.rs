//! Integration coverage for `GET /api/v1/relationships`.
//!
//! Drives the route + the `services::relationships_tree::build_tree`
//! orchestration: three people, two parent links and one partnership,
//! verifying the denormalised `parent_ids` / `partner_ids` adjacency lists
//! and the `linked_user_id` echo.

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

#[allow(clippy::future_not_send)]
async fn create_person<S, B>(
    app: &S,
    access: &str,
    family_id: &str,
    given_name: &str,
    linked_user_id: Option<&str>,
) -> String
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let mut payload = serde_json::json!({ "given_name": given_name });
    if let Some(uid) = linked_user_id {
        payload["linked_user_id"] = serde_json::Value::String(uid.to_string());
    }
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.to_string()))
        .insert_header(("X-Family-Id", family_id.to_string()))
        .set_json(payload)
        .to_request();
    let res = test::call_service(app, req).await;
    assert_eq!(res.status(), 200, "create person `{given_name}` should succeed");
    let body: serde_json::Value = test::read_body_json(res).await;
    body["data"]["id"].as_str().expect("person id").to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relationships_tree_denormalises_parents_and_partners() {
    // Topology:
    //   Mom + Dad -> Kid           (partnership: Mom ↔ Dad; two parent links)
    //   Kid is linked to the signed-in user (linked_user_id is set on Kid)
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "tree-owner@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Trees").await;

    // The owner's user-id, so we can attach it to Kid's `linked_user_id`.
    let owner_email = "tree-owner@example.com";
    let user =
        stack.state.users.find_by_email(owner_email).await.expect("lookup").expect("user exists");
    let owner_uuid = user.id.into_uuid().to_string();

    let mom = create_person(&app, &access, &family_id, "Mom", None).await;
    let dad = create_person(&app, &access, &family_id, "Dad", None).await;
    let kid = create_person(&app, &access, &family_id, "Kid", Some(&owner_uuid)).await;

    // Two parent links.
    for parent in [&mom, &dad] {
        let req = test::TestRequest::post()
            .uri("/api/v1/parent-links")
            .cookie(Cookie::new("access", access.clone()))
            .insert_header(("X-Family-Id", family_id.clone()))
            .set_json(serde_json::json!({
                "child_id": kid,
                "parent_id": parent,
                "kind": "biological",
            }))
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200);
    }

    // Partnership Mom <-> Dad.
    let req = test::TestRequest::post()
        .uri("/api/v1/partnerships")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "partner_a_id": mom,
            "partner_b_id": dad,
            "kind": "marriage",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);

    // GET /relationships and verify the assembled payload.
    let req = test::TestRequest::get()
        .uri("/api/v1/relationships")
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;

    let nodes = body["data"]["nodes"].as_array().expect("nodes array");
    assert_eq!(nodes.len(), 3, "three persons expected");
    let parent_edges = body["data"]["parent_edges"].as_array().expect("parent_edges");
    assert_eq!(parent_edges.len(), 2);
    let partner_edges = body["data"]["partner_edges"].as_array().expect("partner_edges");
    assert_eq!(partner_edges.len(), 1);

    // Find each node by id and assert their adjacency lists.
    let by_id: std::collections::HashMap<String, &serde_json::Value> =
        nodes.iter().map(|n| (n["id"].as_str().unwrap().to_string(), n)).collect();
    let kid_node = by_id.get(&kid).expect("kid node present");
    let mom_node = by_id.get(&mom).expect("mom node present");
    let dad_node = by_id.get(&dad).expect("dad node present");

    // Kid: two parents, no partners, linked_user_id == owner uuid.
    let mut kid_parents: Vec<String> = kid_node["parent_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    kid_parents.sort();
    let mut expected = vec![mom.clone(), dad.clone()];
    expected.sort();
    assert_eq!(kid_parents, expected, "Kid's parents should be Mom and Dad");
    assert!(kid_node["partner_ids"].as_array().unwrap().is_empty());
    assert_eq!(kid_node["linked_user_id"], owner_uuid);

    // Mom and Dad: each has one partner (the other), no parents, no link.
    let mom_partners: Vec<String> = mom_node["partner_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(mom_partners, vec![dad.clone()]);
    let dad_partners: Vec<String> = dad_node["partner_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(dad_partners, vec![mom.clone()]);
    assert!(mom_node["parent_ids"].as_array().unwrap().is_empty());
    assert!(dad_node["parent_ids"].as_array().unwrap().is_empty());
    assert!(mom_node["linked_user_id"].is_null());
    assert!(dad_node["linked_user_id"].is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relationships_tree_empty_family_returns_empty_arrays() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "tree-empty@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Empty").await;

    let req = test::TestRequest::get()
        .uri("/api/v1/relationships")
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert!(body["data"]["nodes"].as_array().unwrap().is_empty());
    assert!(body["data"]["parent_edges"].as_array().unwrap().is_empty());
    assert!(body["data"]["partner_edges"].as_array().unwrap().is_empty());
}
