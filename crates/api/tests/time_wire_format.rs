//! Pins the JSON wire format of dates and timestamps.
//!
//! The backend moved from chrono to `time`, whose default serde format for
//! `OffsetDateTime` is NOT RFC 3339. Every timestamp field is annotated with
//! `time::serde::rfc3339`; these tests fail if one regresses, so the FE
//! contract (`format: date-time` / `format: date` in openapi.json) holds.

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
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// Assert `v` is an RFC 3339 UTC timestamp string (`…T…Z`) and return it parsed.
fn assert_rfc3339(v: &serde_json::Value, what: &str) -> OffsetDateTime {
    let s = v.as_str().unwrap_or_else(|| panic!("{what} should be a string, got {v}"));
    assert!(s.contains('T') && s.ends_with('Z'), "{what} not RFC 3339 UTC: {s}");
    OffsetDateTime::parse(s, &Rfc3339).unwrap_or_else(|e| panic!("{what} unparseable ({e}): {s}"))
}

/// Within a few minutes of now — catches epoch/offset mix-ups.
fn assert_recent(t: OffsetDateTime, what: &str) {
    let age = (OffsetDateTime::now_utc() - t).whole_seconds().abs();
    assert!(age < 300, "{what} should be ~now, is {t}");
}

macro_rules! get_json {
    ($app:expr, $uri:expr, $access:expr) => {
        get_json!($app, $uri, $access, None::<&str>)
    };
    ($app:expr, $uri:expr, $access:expr, $family:expr) => {{
        let mut req = test::TestRequest::get().uri($uri).cookie(Cookie::new("access", $access));
        if let Some(f) = $family {
            req = req.insert_header(("X-Family-Id", f.to_string()));
        }
        let res = test::call_service($app, req.to_request()).await;
        let status = res.status().as_u16();
        let body: serde_json::Value = test::read_body_json(res).await;
        (status, body)
    }};
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn timestamps_serialize_as_rfc3339() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "wire@example.com").await;
    let (access, family_id) = create_family(&app, &access, "WireFam").await;

    // users/me: created_at + (verified by magic-link sign-in) email_verified_at.
    let (status, me) = get_json!(&app, "/api/v1/users/me", access.clone());
    assert_eq!(status, 200, "{me}");
    assert_recent(assert_rfc3339(&me["data"]["created_at"], "user.created_at"), "user.created_at");
    assert_rfc3339(&me["data"]["email_verified_at"], "user.email_verified_at");

    // families/me: per-family created_at.
    let (status, fams) = get_json!(&app, "/api/v1/families/me", access.clone());
    assert_eq!(status, 200, "{fams}");
    let fam = &fams["data"]["families"].as_array().expect("families array")[0];
    assert_recent(assert_rfc3339(&fam["created_at"], "family.created_at"), "family.created_at");

    // members: joined_at.
    let uri = format!("/api/v1/families/{family_id}/members");
    let (status, members) = get_json!(&app, &uri, access.clone(), Some(&family_id));
    assert_eq!(status, 200, "{members}");
    let m = &members["data"]["data"].as_array().expect("members array")[0];
    assert_recent(assert_rfc3339(&m["joined_at"], "member.joined_at"), "member.joined_at");

    // audit: created_at on each row (create_family wrote one).
    let uri = format!("/api/v1/families/{family_id}/audit");
    let (status, audit) = get_json!(&app, &uri, access.clone(), Some(&family_id));
    assert_eq!(status, 200, "{audit}");
    let row = &audit["data"]["data"].as_array().expect("audit rows")[0];
    assert_recent(assert_rfc3339(&row["created_at"], "audit.created_at"), "audit.created_at");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn calendar_dates_round_trip_as_plain_iso_dates() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "dates@example.com").await;
    let (access, family_id) = create_family(&app, &access, "DateFam").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "given_name": "Old",
            "family_name": "Ancestor",
            "birth_date": "1870-12-03",
            "death_date": "1950-02-28",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["birth_date"], "1870-12-03");
    assert_eq!(body["data"]["death_date"], "1950-02-28");

    // An impossible date is rejected at deserialization, not stored.
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id))
        .set_json(serde_json::json!({
            "given_name": "Bad",
            "family_name": "Date",
            "birth_date": "1990-02-30",
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status().as_u16(), 400, "Feb 30 must be rejected");
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "request_malformed", "{body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_range_filter_accepts_rfc3339_and_rejects_other_formats() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "audit-range@example.com").await;
    let (access, family_id) = create_family(&app, &access, "RangeFam").await;
    let base = format!("/api/v1/families/{family_id}/audit");

    // A window around now contains create_family's row…
    let uri = format!("{base}?from=2000-01-01T00:00:00Z&to=2999-01-01T00:00:00Z");
    let (status, body) = get_json!(&app, &uri, access.clone(), Some(&family_id));
    assert_eq!(status, 200, "{body}");
    assert!(body["data"]["total"].as_i64().unwrap() >= 1, "{body}");

    // …a window entirely in the past does not. Offsets other than Z parse too.
    let uri = format!("{base}?from=2000-01-01T00:00:00%2B02:00&to=2000-01-02T00:00:00Z");
    let (status, body) = get_json!(&app, &uri, access.clone(), Some(&family_id));
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["data"]["total"].as_i64().unwrap(), 0, "{body}");

    // A bare date is not a timestamp: rejected rather than silently ignored.
    let uri = format!("{base}?from=2026-01-01");
    let (status, body) = get_json!(&app, &uri, access, Some(&family_id));
    assert_eq!(status, 400, "date-only `from` must be rejected");
    assert_eq!(body["code"], "request_malformed", "problem+json, not plain text: {body}");
}
