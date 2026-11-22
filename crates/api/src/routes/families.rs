//! `/families` CRUD + `/families/{id}/invites` endpoints.
//!
//! - `GET    /families/me`             — list every family the caller belongs
//!   to. Reads memberships fresh from the DB (not the JWT) so it can surface
//!   each family's `created_at` for the switcher without bloating the token.
//! - `POST   /families`                — create a new family, auto-join the
//!   creator as `Owner`, then reissue a fresh access cookie so the new
//!   membership is immediately reflected in the JWT.
//! - `PATCH  /families/{id}`           — rename. Requires `Admin` or `Owner`
//!   on the target family.
//! - `DELETE /families/{id}`           — delete. Requires `Owner` on the
//!   target family. Returns `{ "data": null }` per spec §5.
//! - `POST   /families/{id}/invites`   — issue a single-use invite token,
//!   email it. Requires `Admin` or `Owner`. Cannot invite as `Owner`.
//!
//! Authorisation pattern: every per-family handler resolves the target id
//! against `claims.all_families` first (returning `NotFamilyMember` when the
//! caller isn't in the token), then [`crate::auth::require_role`] against the
//! resolved membership. We deliberately do NOT require the `X-Family-Id`
//! header here: these handlers identify their target via the path segment,
//! and the header is reserved for handlers whose target family is implicit
//! (persons, contacts, reminders).

mod invites;
mod manage;
mod overview;

pub use invites::{__path_invite, invite};
pub use manage::{
    __path_create, __path_delete_family, __path_list_mine, __path_rename, create, delete_family,
    list_mine, rename,
};
use my_fam_tree_domain::Role;
pub use overview::{__path_admin_overview, admin_overview};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::ActiveFamily;
use crate::validation::{string_too_long, value_required};
use crate::{ApiError, response_body};

// Family-name cap (security audit MEDIUM). The same value flows verbatim
// into the invite + owner-transfer email Subject lines (`templates.rs`);
// lettre escapes CR/LF in headers, so this is the second line of defense
// against header-injection on the off-chance the template path changes.
const FAMILY_NAME_MAX: u32 = 200;

/// Trim and validate a family name. Rejects empty, over-long, or
/// strings containing CR/LF (header-injection hardening).
fn validate_family_name(raw: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(value_required("/name"));
    }
    if trimmed.contains('\r') || trimmed.contains('\n') {
        return Err(ApiError::Validation(vec![crate::error::FieldViolation::new(
            "/name",
            "validation.value_invalid",
            "family name must not contain newline characters",
        )]));
    }
    if u32::try_from(trimmed.chars().count()).unwrap_or(u32::MAX) > FAMILY_NAME_MAX {
        return Err(string_too_long("/name", FAMILY_NAME_MAX));
    }
    Ok(trimmed.to_owned())
}

// ---------------------------------------------------------------------------
// Request / response DTOs.
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct FamilyView {
    pub id: Uuid,
    pub name: String,
    pub role: Role,
    /// Family creation time (RFC 3339). Present on `GET /families/me` so the
    /// switcher can disambiguate same-named families by date; `None` on the
    /// JWT-projection paths (rename, invite-accept) that don't carry it.
    #[serde(skip_serializing_if = "Option::is_none", with = "time::serde::rfc3339::option")]
    pub created_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MyFamiliesRes {
    pub families: Vec<FamilyView>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateFamilyReq {
    pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateFamilyRes {
    pub family: FamilyView,
    pub claims: super::auth::ConsumeRes,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RenameFamilyReq {
    pub name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InviteReq {
    pub email: String,
    pub role: Role,
    /// Optional person row this invite is bound to. On accept, the API
    /// atomically sets `persons.linked_user_id = new_user.id` so the
    /// recipient becomes the person they were invited as. Nullable: an
    /// admin may invite without binding to a specific person.
    #[serde(default)]
    pub person_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InviteRes {
    pub status: &'static str,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LatestPersonView {
    pub id: Uuid,
    pub given_name: String,
    pub family_name: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Aggregated stats for the admin "Family" overview page.
///
/// Single round-trip so the page doesn't need to fan out to three
/// separate endpoints (count members, count persons, fetch latest 3
/// persons). Returned by `GET /admin/family/overview`.
#[derive(Debug, Serialize, ToSchema)]
pub struct FamilyOverview {
    pub id: Uuid,
    pub name: String,
    pub role: Role,
    pub member_count: u64,
    pub person_count: u64,
    /// At most 3 entries, ordered newest-first by `created_at`.
    pub latest_persons: Vec<LatestPersonView>,
}

response_body!(pub MyFamiliesResponseBody, MyFamiliesRes);
response_body!(pub CreateFamilyResponseBody, CreateFamilyRes);
response_body!(pub FamilyViewResponseBody, FamilyView);
response_body!(pub InviteResponseBody, InviteRes);
response_body!(pub FamilyOverviewResponseBody, FamilyOverview);

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

/// Find `family_id` among the caller's JWT memberships, returning a thin
/// `ActiveFamily` snapshot the role checker can consume.
pub(crate) fn resolve_membership(
    claims: &crate::auth::UserClaims,
    family_id: Uuid,
) -> Result<ActiveFamily, ApiError> {
    claims
        .all_families
        .iter()
        .find(|f| f.id.into_uuid() == family_id)
        .map(|f| ActiveFamily { id: f.id, name: f.name.clone(), role: f.role })
        .ok_or(ApiError::NotFamilyMember(family_id))
}

fn seconds_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

// ---------------------------------------------------------------------------
// Tests — pure-logic helpers; HTTP integration tests live in tests/.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use my_fam_tree_domain::{FamilyId, Role, UserId};
    use uuid::Uuid;

    use super::*;
    use crate::auth::user_claims::{FamilyMembershipMirror, UserClaims};

    fn mk_claims(memberships: Vec<(Uuid, &'static str, Role)>) -> UserClaims {
        UserClaims {
            user_id: UserId::from_uuid(Uuid::new_v4()),
            email: "a@b.c".into(),
            locale: "en".into(),
            active_family: None,
            all_families: memberships
                .into_iter()
                .map(|(id, name, role)| FamilyMembershipMirror {
                    id: FamilyId::from_uuid(id),
                    name: name.into(),
                    role,
                })
                .collect(),
        }
    }

    #[test]
    fn resolve_membership_returns_active_family_when_present() {
        let fid = Uuid::new_v4();
        let claims = mk_claims(vec![(fid, "Müller", Role::Owner)]);
        let active = resolve_membership(&claims, fid).expect("resolved");
        assert_eq!(active.id.into_uuid(), fid);
        assert_eq!(active.name, "Müller");
        assert_eq!(active.role, Role::Owner);
    }

    #[test]
    fn resolve_membership_errors_when_not_in_token() {
        let claims = mk_claims(vec![(Uuid::new_v4(), "Müller", Role::Owner)]);
        let missing = Uuid::new_v4();
        match resolve_membership(&claims, missing) {
            Err(ApiError::NotFamilyMember(id)) => assert_eq!(id, missing),
            _ => panic!("expected NotFamilyMember"),
        }
    }

    #[test]
    fn seconds_i64_clamps_overflow_to_max() {
        assert_eq!(seconds_i64(0), 0);
        assert_eq!(seconds_i64(900), 900);
        assert_eq!(seconds_i64(u64::MAX), i64::MAX);
    }

    #[test]
    fn invite_res_serialises_with_status_field() {
        let v = serde_json::to_value(InviteRes { status: "sent" }).unwrap();
        assert_eq!(v["status"], "sent");
    }

    #[test]
    fn create_family_req_deserialises_from_name_field() {
        let r: CreateFamilyReq =
            serde_json::from_value(serde_json::json!({"name": "Müller"})).unwrap();
        assert_eq!(r.name, "Müller");
    }

    #[test]
    fn invite_req_deserialises_email_and_role() {
        let r: InviteReq =
            serde_json::from_value(serde_json::json!({"email": "a@b.co", "role": "admin"}))
                .unwrap();
        assert_eq!(r.email, "a@b.co");
        assert_eq!(r.role, Role::Admin);
        assert!(r.person_id.is_none());
    }

    #[test]
    fn invite_req_deserialises_with_person_id() {
        let pid = Uuid::new_v4();
        let r: InviteReq = serde_json::from_value(
            serde_json::json!({"email": "a@b.co", "role": "user", "person_id": pid}),
        )
        .unwrap();
        assert_eq!(r.person_id, Some(pid));
    }

    #[test]
    fn family_view_serialises_with_role_as_string() {
        let v = serde_json::to_value(FamilyView {
            id: Uuid::nil(),
            name: "Müller".into(),
            role: Role::Owner,
            created_at: None,
        })
        .unwrap();
        assert_eq!(v["name"], "Müller");
        assert!(v.get("role").is_some());
    }
}
