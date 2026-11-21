//! Shared fixtures for the invite integration tests
//! (`invites_flow.rs` + `invites_admin_flow.rs`).

use my_fam_tree_domain::{FamilyId, PersonDraft, PersonId};
use uuid::Uuid;

use super::{TestStack, create_family, sign_in};

/// Seed an admin-owned family with one person (Klaus) and return
/// `(access_cookie, family_uuid_string, klaus_person_id)` for the caller
/// to drive the invite flow from.
#[expect(
    clippy::future_not_send,
    reason = "actix test service futures are !Send; tests run them on the current thread"
)]
pub async fn seed_admin_family_with_person<S, B>(
    stack: &TestStack,
    app: &S,
    stamp: u128,
) -> (String, String, PersonId)
where
    S: actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        >,
    B: actix_web::body::MessageBody,
{
    let owner_email = format!("invite-owner-{stamp}@example.com");
    let (access, _r) = sign_in(stack, app, &owner_email).await;
    let (access, family_id_str) = create_family(app, &access, &format!("InviteFam-{stamp}")).await;
    let family_uuid: Uuid = family_id_str.parse().expect("uuid");
    let family_id = FamilyId::from_uuid(family_uuid);

    // Drain the magic-link email so callers can inspect invite mails by
    // grabbing `.last()` on the captured drain.
    stack.fake_email.drain();

    let klaus = stack
        .state
        .persons
        .create(
            family_id,
            PersonDraft {
                given_name: "Klaus".into(),
                family_name: "Müller".into(),
                ..PersonDraft::default()
            },
        )
        .await
        .expect("create klaus");

    (access, family_id_str, klaus.id)
}
