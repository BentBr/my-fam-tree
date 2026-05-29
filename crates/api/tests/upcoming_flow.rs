//! Integration coverage for `GET /api/v1/upcoming`.
//!
//! Builds a small family graph with persons (with mixed birth +
//! death dates) and a couple of partnerships (one open, one ended),
//! then exercises the filter / sort / limit behaviour against the
//! real Postgres + Redis stack via testcontainers.

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
use chrono::{Datelike, Utc};
use common::{create_family, ephemeral_stack, sign_in};
use my_fam_tree_api::build_app;

/// Helper: POST a person with optional birth/death dates and return the new id.
#[allow(clippy::future_not_send, clippy::too_many_arguments)]
async fn create_person<S, B>(
    app: &S,
    access: &str,
    family_id: &str,
    given_name: &str,
    family_name: &str,
    birth_date: Option<&str>,
    death_date: Option<&str>,
) -> String
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let mut body = serde_json::json!({
        "given_name": given_name,
        "family_name": family_name,
    });
    if let Some(d) = birth_date {
        body["birth_date"] = serde_json::Value::String(d.to_owned());
    }
    if let Some(d) = death_date {
        body["death_date"] = serde_json::Value::String(d.to_owned());
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

#[allow(clippy::future_not_send)]
async fn create_partnership<S, B>(
    app: &S,
    access: &str,
    family_id: &str,
    a: &str,
    b: &str,
    started_on: Option<&str>,
    ended_on: Option<&str>,
) -> String
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let mut body = serde_json::json!({
        "partner_a_id": a,
        "partner_b_id": b,
        "kind": "marriage",
    });
    if let Some(d) = started_on {
        body["started_on"] = serde_json::Value::String(d.to_owned());
    }
    if let Some(d) = ended_on {
        body["ended_on"] = serde_json::Value::String(d.to_owned());
        body["end_reason"] = serde_json::Value::String("divorce".to_owned());
    }
    let req = test::TestRequest::post()
        .uri("/api/v1/partnerships")
        .cookie(Cookie::new("access", access.to_string()))
        .insert_header(("X-Family-Id", family_id.to_string()))
        .set_json(body)
        .to_request();
    let res = test::call_service(app, req).await;
    assert_eq!(res.status(), 200, "create partnership should succeed");
    let body: serde_json::Value = test::read_body_json(res).await;
    body["data"]["id"].as_str().expect("partnership id").to_string()
}

/// All three filter modes share the same seed so we only build the
/// graph once. Open partnership ⇒ `wedding_anniversary`; ended
/// partnership ⇒ no event. Death-this-year is suppressed.
#[allow(
    clippy::too_many_lines,
    reason = "single end-to-end scenario covering all three filter modes"
)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upcoming_filter_modes_sort_and_limit() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "upcoming-owner@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Upcoming").await;

    // Pick dates relative to today so the test stays time-stable
    // year over year. Use birthdays in distinct months so we can
    // assert sort order without ambiguity.
    let alice =
        create_person(&app, &access, &family_id, "Alice", "Müller", Some("1980-06-04"), None).await;
    let bob =
        create_person(&app, &access, &family_id, "Bob", "Müller", Some("1982-08-22"), None).await;
    // Carl has both a birth_date AND a death_date in a prior year ⇒
    // contributes both `birthday` and `death_anniversary` events.
    let carl = create_person(
        &app,
        &access,
        &family_id,
        "Carl",
        "Müller",
        Some("1950-09-12"),
        Some("2020-09-13"),
    )
    .await;
    // Dora died THIS year ⇒ death_anniversary must be suppressed.
    let this_year = Utc::now().date_naive().year();
    let dora_death = format!("{this_year}-12-31");
    let dora = create_person(
        &app,
        &access,
        &family_id,
        "Dora",
        "Müller",
        Some("1955-10-10"),
        Some(&dora_death),
    )
    .await;
    // Eve has no birth_date ⇒ contributes nothing.
    let _eve = create_person(&app, &access, &family_id, "Eve", "Müller", None, None).await;

    // Open partnership: Alice + Bob ⇒ wedding_anniversary.
    let _ab =
        create_partnership(&app, &access, &family_id, &alice, &bob, Some("2005-07-15"), None).await;
    // Ended partnership: Carl + Dora ⇒ NO wedding_anniversary.
    let _cd = create_partnership(
        &app,
        &access,
        &family_id,
        &carl,
        &dora,
        Some("1980-02-14"),
        Some("2010-05-01"),
    )
    .await;

    // --- filter=all (default) ---
    let req = test::TestRequest::get()
        .uri("/api/v1/upcoming")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let rows = body["data"].as_array().expect("array").clone();
    // 4 birthdays (Alice, Bob, Carl, Dora) + 1 wedding (Alice+Bob)
    // + 1 death_anniv (Carl). Dora's death is this year ⇒ skipped.
    assert_eq!(rows.len(), 6, "all-filter should return 6 events: {rows:?}");
    let kinds: Vec<&str> = rows.iter().map(|r| r["kind"].as_str().unwrap()).collect();
    assert!(kinds.contains(&"birthday"));
    assert!(kinds.contains(&"wedding_anniversary"));
    assert!(kinds.contains(&"death_anniversary"));
    // Sort ascending by next_date.
    let dates: Vec<&str> = rows.iter().map(|r| r["next_date"].as_str().unwrap()).collect();
    let mut sorted = dates.clone();
    sorted.sort_unstable();
    assert_eq!(dates, sorted, "events must be sorted by next_date ascending");

    // --- filter=birthday ---
    let req = test::TestRequest::get()
        .uri("/api/v1/upcoming?filter=birthday")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let rows = body["data"].as_array().unwrap().clone();
    assert_eq!(rows.len(), 4, "birthday-filter should return 4 birthdays");
    for r in &rows {
        assert_eq!(r["kind"], "birthday");
    }

    // --- filter=anniversary ---
    let req = test::TestRequest::get()
        .uri("/api/v1/upcoming?filter=anniversary")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let rows = body["data"].as_array().unwrap().clone();
    // 1 wedding + 1 death (Carl). Dora's death this year ⇒ skipped.
    // Ended Carl+Dora partnership ⇒ no wedding.
    assert_eq!(rows.len(), 2, "anniversary-filter should return 2 events: {rows:?}");
    for r in &rows {
        let k = r["kind"].as_str().unwrap();
        assert!(
            k == "wedding_anniversary" || k == "death_anniversary",
            "anniversary filter must not contain birthdays: {k}"
        );
    }
    let mut kinds: Vec<&str> = rows.iter().map(|r| r["kind"].as_str().unwrap()).collect();
    kinds.sort_unstable();
    assert_eq!(kinds, vec!["death_anniversary", "wedding_anniversary"]);

    // --- limit=2 ---
    let req = test::TestRequest::get()
        .uri("/api/v1/upcoming?limit=2")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let rows = body["data"].as_array().unwrap().clone();
    assert_eq!(rows.len(), 2, "limit=2 should truncate to 2 events");
}

/// Sanity check: when only birthdays exist, the anniversary filter
/// returns an empty list and the all filter returns just the
/// birthdays.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn upcoming_empty_anniversary_when_no_partnership_no_death() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "upcoming-empty@example.com").await;
    let (access, family_id) = create_family(&app, &access, "Birthdays").await;
    let _ =
        create_person(&app, &access, &family_id, "Solo", "Person", Some("1990-01-15"), None).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/upcoming?filter=anniversary")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert!(body["data"].as_array().unwrap().is_empty());

    let req = test::TestRequest::get()
        .uri("/api/v1/upcoming")
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let rows = body["data"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["kind"], "birthday");
    // Birthday years field is positive (won't be 0).
    assert!(rows[0]["years"].as_u64().unwrap() >= 1);
}
