//! Integration coverage for `/api/v1/persons/{id}/contacts` and
//! `/api/v1/contacts/{id}` — the Phase 3 replacement for the flat
//! `persons.email/phone/...` columns. Exercises every `ContactKind`,
//! the `family` vs `admins_only` visibility filter, and the role
//! gates (admin/owner vs `user` linked to own person vs `user`
//! against someone else's contact).

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn person_contacts_every_kind_round_trips() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _refresh) = sign_in(&stack, &app, "owner-contacts@example.com").await;
    let (access, family_id) = create_family(&app, &access, "ContactsFam").await;

    // Create a person to hang contacts on.
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({ "given_name": "Klaus", "family_name": "Müller" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let person_id = body["data"]["id"].as_str().unwrap().to_string();

    // Post each of the five `kind`s. The address row carries the full
    // postal structure that used to live in flat columns; everything
    // else is `{ "v": "..." }`.
    let inputs: Vec<(&str, serde_json::Value)> = vec![
        (
            "email",
            serde_json::json!({
                "kind": "email",
                "label": "Work",
                "value": { "email": "klaus@example.com" },
                "visibility": "family",
            }),
        ),
        (
            "phone",
            serde_json::json!({
                "kind": "phone",
                "label": "Mobile",
                "value": { "number": "+49 40 5550101" },
            }),
        ),
        (
            "address",
            serde_json::json!({
                "kind": "address",
                "label": "Home",
                "value": {
                    "street": "Mittelweg",
                    "house_number": "12",
                    "zip": "20148",
                    "city": "Hamburg",
                    "country": "Deutschland",
                },
            }),
        ),
        (
            "url",
            serde_json::json!({
                "kind": "url",
                "label": "Homepage",
                "value": { "url": "https://klaus.example.de" },
            }),
        ),
        (
            "other",
            serde_json::json!({
                "kind": "other",
                "label": "Funkgerät",
                "value": { "text": "DK4ZA" },
            }),
        ),
    ];
    for (kind, body) in &inputs {
        let req = test::TestRequest::post()
            .uri(&format!("/api/v1/persons/{person_id}/contacts"))
            .cookie(Cookie::new("access", access.clone()))
            .insert_header(("X-Family-Id", family_id.clone()))
            .set_json(body)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200, "kind={kind} should accept");
    }

    // GET list — all 5 round-trip with their kind + value intact.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/persons/{person_id}/contacts"))
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let rows = body["data"]["contacts"].as_array().unwrap();
    assert_eq!(rows.len(), 5);

    // Address: structured postal fields survived the JSONB round-trip.
    let address = rows.iter().find(|r| r["kind"] == "address").unwrap();
    assert_eq!(address["value"]["street"], "Mittelweg");
    assert_eq!(address["value"]["house_number"], "12");
    assert_eq!(address["value"]["zip"], "20148");
    assert_eq!(address["value"]["city"], "Hamburg");
    assert_eq!(address["value"]["country"], "Deutschland");
    assert_eq!(address["label"], "Home");
    assert_eq!(address["visibility"], "family");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn person_contacts_admins_only_visibility_hidden_from_user_role() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (owner_access, _r) = sign_in(&stack, &app, "owner-vis@example.com").await;
    let (owner_access, family_id) = create_family(&app, &owner_access, "VisFam").await;
    let fam_uuid = Uuid::parse_str(&family_id).expect("uuid");

    // Owner creates a person + two contacts: one family, one admins_only.
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", owner_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({ "given_name": "Anna", "family_name": "Schmidt" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    let person_id = body["data"]["id"].as_str().unwrap().to_string();

    for body in [
        serde_json::json!({
            "kind": "email",
            "value": { "email": "anna@example.com" },
            "visibility": "family",
        }),
        serde_json::json!({
            "kind": "email",
            "label": "Private",
            "value": { "email": "anna-private@example.com" },
            "visibility": "admins_only",
        }),
    ] {
        let req = test::TestRequest::post()
            .uri(&format!("/api/v1/persons/{person_id}/contacts"))
            .cookie(Cookie::new("access", owner_access.clone()))
            .insert_header(("X-Family-Id", family_id.clone()))
            .set_json(body)
            .to_request();
        let res = test::call_service(&app, req).await;
        assert_eq!(res.status(), 200);
    }

    // Add a regular `user`-role member to the family.
    let regular_email = "vis-regular@example.com";
    let _ = sign_in(&stack, &app, regular_email).await;
    let user =
        stack.state.users.find_by_email(regular_email).await.expect("lookup").expect("user exists");
    stack
        .state
        .memberships
        .insert(FamilyId::from_uuid(fam_uuid), user.id, Role::User)
        .await
        .expect("membership insert");
    let (user_access, _r) = sign_in(&stack, &app, regular_email).await;

    // Owner sees both contacts.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/persons/{person_id}/contacts"))
        .cookie(Cookie::new("access", owner_access))
        .insert_header(("X-Family-Id", family_id.clone()))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["data"]["contacts"].as_array().unwrap().len(), 2);

    // `user` role sees ONLY the `family` contact; `admins_only` is filtered.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v1/persons/{person_id}/contacts"))
        .cookie(Cookie::new("access", user_access))
        .insert_header(("X-Family-Id", family_id))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    let rows = body["data"]["contacts"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["visibility"], "family");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn person_contacts_user_role_edit_gate() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (owner_access, _r) = sign_in(&stack, &app, "owner-edit@example.com").await;
    let (owner_access, family_id) = create_family(&app, &owner_access, "EditFam").await;
    let fam_uuid = Uuid::parse_str(&family_id).expect("uuid");

    // Owner creates one person of their own.
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", owner_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({ "given_name": "Owner Person" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    let other_person_id = body["data"]["id"].as_str().unwrap().to_string();

    // Sign the guest in so the user row exists, then add their membership.
    let guest_email = "edit-guest@example.com";
    let _ = sign_in(&stack, &app, guest_email).await;
    let guest_user =
        stack.state.users.find_by_email(guest_email).await.expect("lookup").expect("user");
    stack
        .state
        .memberships
        .insert(FamilyId::from_uuid(fam_uuid), guest_user.id, Role::User)
        .await
        .expect("membership insert");

    // Owner creates a person linked to the guest user.
    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", owner_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "given_name": "Guest Person",
            "linked_user_id": guest_user.id.into_uuid(),
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    let guest_person_id = body["data"]["id"].as_str().unwrap().to_string();

    // Owner seeds a contact on the OWNER person so the guest has something
    // to be blocked from editing.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/persons/{other_person_id}/contacts"))
        .cookie(Cookie::new("access", owner_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "kind": "email",
            "value": { "email": "owner@example.com" },
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let other_contact_id = body["data"]["id"].as_str().unwrap().to_string();

    // Guest signs in fresh and exercises the gate.
    let (guest_access, _r) = sign_in(&stack, &app, guest_email).await;

    // OK: guest creates a contact on their OWN person.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/persons/{guest_person_id}/contacts"))
        .cookie(Cookie::new("access", guest_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "kind": "phone",
            "value": { "number": "+49 30 5550199" },
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = test::read_body_json(res).await;
    let own_contact_id = body["data"]["id"].as_str().unwrap().to_string();

    // OK: guest updates their own contact.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/contacts/{own_contact_id}"))
        .cookie(Cookie::new("access", guest_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "kind": "phone",
            "value": { "number": "+49 30 5550100" },
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 200);

    // BLOCKED: guest tries to PATCH a contact on the OWNER's person.
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v1/contacts/{other_contact_id}"))
        .cookie(Cookie::new("access", guest_access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({
            "kind": "email",
            "value": { "email": "evil@example.com" },
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "contact_not_editable");

    // BLOCKED: guest tries to DELETE a contact on the OWNER's person.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v1/contacts/{other_contact_id}"))
        .cookie(Cookie::new("access", guest_access))
        .insert_header(("X-Family-Id", family_id))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 403);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn person_contacts_email_kind_rejects_malformed_address() {
    let stack = ephemeral_stack().await;
    let app = test::init_service(build_app(stack.state.clone(), None)).await;
    let (access, _r) = sign_in(&stack, &app, "owner-email@example.com").await;
    let (access, family_id) = create_family(&app, &access, "EmailFam").await;

    let req = test::TestRequest::post()
        .uri("/api/v1/persons")
        .cookie(Cookie::new("access", access.clone()))
        .insert_header(("X-Family-Id", family_id.clone()))
        .set_json(serde_json::json!({ "given_name": "Test" }))
        .to_request();
    let res = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(res).await;
    let person_id = body["data"]["id"].as_str().unwrap().to_string();

    // Garbage in `value.v` for an email kind ⇒ 422.
    let req = test::TestRequest::post()
        .uri(&format!("/api/v1/persons/{person_id}/contacts"))
        .cookie(Cookie::new("access", access))
        .insert_header(("X-Family-Id", family_id))
        .set_json(serde_json::json!({
            "kind": "email",
            "value": { "email": "not-an-email" },
        }))
        .to_request();
    let res = test::call_service(&app, req).await;
    assert_eq!(res.status(), 422);
    let body: serde_json::Value = test::read_body_json(res).await;
    assert_eq!(body["code"], "validation_failed");
}
