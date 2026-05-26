//! `/families` CRUD + `/families/{id}/invites` endpoints.
//!
//! - `GET    /families/me`             — echo every family the JWT proves the
//!   caller belongs to. No DB round-trip; reads from the request's `UserClaims`.
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

use actix_web::{HttpRequest, HttpResponse, delete, get, patch, post, web};
use chrono::{Duration, Utc};
use my_family_domain::{FamilyId, Role};
use my_family_email::{Locale as EmailLocale, OutboundEmail, render_invite};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::{ActiveFamily, FamilyClaim, generate_opaque_token};
use crate::cookies::access_cookie;
use crate::response::NullResponseBody;
use crate::services::audit;
use crate::services::auth_service::issue_access_token_for;
use crate::validation::{email_invalid, looks_like_email, role_invalid, value_required};
use crate::{ApiError, ApiResponse, AppState, response_body};

// ---------------------------------------------------------------------------
// Request / response DTOs.
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct FamilyView {
    pub id: Uuid,
    pub name: String,
    pub role: Role,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MyFamiliesRes {
    pub families: Vec<FamilyView>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateFamilyReq {
    pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateFamilyRes {
    pub family: FamilyView,
    pub claims: super::auth::ConsumeRes,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RenameFamilyReq {
    pub name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
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

/// Wire DTO returned by `GET /families/{id}/invites`. Mirrors the persisted
/// `Invite` minus the token hash (which never leaves the database).
#[derive(Debug, Serialize, ToSchema)]
pub struct InviteDto {
    pub id: Uuid,
    pub email: String,
    pub role: Role,
    pub person_id: Option<Uuid>,
    pub expires_at: chrono::DateTime<Utc>,
    pub invited_by: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InvitesList {
    pub data: Vec<InviteDto>,
}

response_body!(pub MyFamiliesResponseBody, MyFamiliesRes);
response_body!(pub CreateFamilyResponseBody, CreateFamilyRes);
response_body!(pub FamilyViewResponseBody, FamilyView);
response_body!(pub InviteResponseBody, InviteRes);
response_body!(pub InvitesListResponseBody, InvitesList);

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

/// Find `family_id` among the caller's JWT memberships, returning a thin
/// `ActiveFamily` snapshot the role checker can consume.
fn resolve_membership(
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
// GET /families/me
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/families/me",
    responses(
        (status = 200, description = "Caller's family memberships", body = MyFamiliesResponseBody),
        (status = 401, description = "No session"),
    ),
    security(("cookie_access" = [])),
    tag = "families",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[get("/families/me")]
pub async fn list_mine(req: HttpRequest) -> Result<ApiResponse<MyFamiliesRes>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let families = claims
        .all_families
        .into_iter()
        .map(|f| FamilyView { id: f.id.into_uuid(), name: f.name, role: f.role })
        .collect();
    Ok(ApiResponse::ok(MyFamiliesRes { families }))
}

// ---------------------------------------------------------------------------
// POST /families
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/families",
    operation_id = "families_create",
    request_body = CreateFamilyReq,
    responses(
        (status = 200, description = "Family created and caller auto-joined as Owner", body = CreateFamilyResponseBody),
        (status = 401, description = "No session"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "families",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/families")]
pub async fn create(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateFamilyReq>,
) -> Result<HttpResponse, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let name = body.name.trim();
    if name.is_empty() {
        return Err(value_required("/name"));
    }

    let family = state.families.create(name, claims.user_id).await.map_err(internal)?;
    state.memberships.insert(family.id, claims.user_id, Role::Owner).await.map_err(internal)?;
    audit::record(
        &state.audit,
        family.id,
        claims.user_id,
        "create",
        "membership",
        None,
        serde_json::json!({
            "user_id": claims.user_id.into_uuid(),
            "role": "owner",
        }),
    )
    .await;

    // Reissue access JWT so the new membership is visible immediately.
    let user = state
        .users
        .find_by_id(claims.user_id)
        .await
        .map_err(internal)?
        .ok_or(ApiError::Unauthenticated)?;
    let (access, fams) = issue_access_token_for(&state.jwt_issuer, &state.memberships, &user)
        .await
        .map_err(ApiError::Internal)?;

    let claims_payload = super::auth::ConsumeRes {
        user_id: user.id.into_uuid(),
        email: user.email.clone(),
        locale: user.locale.as_str().to_string(),
        families: fams,
    };
    let response = CreateFamilyRes {
        family: FamilyView { id: family.id.into_uuid(), name: family.name, role: Role::Owner },
        claims: claims_payload,
    };

    let mut resp = HttpResponse::Ok().json(ApiResponse::ok(response));
    let _ = resp.add_cookie(&access_cookie(&state.cfg, access));
    Ok(resp)
}

// ---------------------------------------------------------------------------
// PATCH /families/{id}
// ---------------------------------------------------------------------------

#[utoipa::path(
    patch,
    path = "/api/v1/families/{id}",
    request_body = RenameFamilyReq,
    responses(
        (status = 200, description = "Family renamed", body = FamilyViewResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Insufficient role"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "families",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[patch("/families/{id}")]
pub async fn rename(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<RenameFamilyReq>,
) -> Result<ApiResponse<FamilyView>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let id = path.into_inner();
    let active = resolve_membership(&claims, id)?;
    crate::auth::require_role(&active, Role::Admin)?;

    let name = body.name.trim();
    if name.is_empty() {
        return Err(value_required("/name"));
    }
    state.families.rename(FamilyId::from_uuid(id), name).await.map_err(internal)?;
    Ok(ApiResponse::ok(FamilyView { id, name: name.to_string(), role: active.role }))
}

// ---------------------------------------------------------------------------
// DELETE /families/{id}
// ---------------------------------------------------------------------------

#[utoipa::path(
    delete,
    path = "/api/v1/families/{id}",
    responses(
        (status = 200, description = "Family deleted", body = NullResponseBody, content_type = "application/json"),
        (status = 401, description = "No session"),
        (status = 403, description = "Insufficient role"),
    ),
    security(("cookie_access" = [])),
    tag = "families",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[delete("/families/{id}")]
pub async fn delete_family(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<ApiResponse<serde_json::Value>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let id = path.into_inner();
    let active = resolve_membership(&claims, id)?;
    crate::auth::require_role(&active, Role::Owner)?;
    state.families.delete(FamilyId::from_uuid(id)).await.map_err(internal)?;
    Ok(ApiResponse::ok(serde_json::Value::Null))
}

// ---------------------------------------------------------------------------
// POST /families/{id}/invites
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/families/{id}/invites",
    request_body = InviteReq,
    responses(
        (status = 200, description = "Invite created and email queued", body = InviteResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Insufficient role"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "families",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/families/{id}/invites")]
pub async fn invite(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<InviteReq>,
) -> Result<ApiResponse<InviteRes>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let family_id = path.into_inner();
    let active = resolve_membership(&claims, family_id)?;
    crate::auth::require_role(&active, Role::Admin)?;

    if body.role == Role::Owner {
        return Err(role_invalid("/role", "cannot invite as owner"));
    }

    let email = body.email.trim().to_lowercase();
    if !looks_like_email(&email) {
        return Err(email_invalid("/email"));
    }

    // Reject duplicates BEFORE generating a token / sending email. Two
    // pending invites for the same email in the same family would race on
    // accept (only one can match the freshly-signed-in JWT email) and
    // pollute the admin pending-list with stale rows.
    let fid = FamilyId::from_uuid(family_id);
    if state.invites.find_pending_by_email(fid, &email).await.map_err(internal)?.is_some() {
        return Err(ApiError::InviteDuplicate);
    }

    let (token, hash) = generate_opaque_token();
    let invite_id = state
        .invites
        .create(
            fid,
            &email,
            body.role,
            claims.user_id,
            body.person_id,
            &hash,
            Utc::now() + Duration::seconds(seconds_i64(state.cfg.invite_ttl_seconds)),
        )
        .await
        .map_err(internal)?;

    let link = format!("{}/invite/accept?token={}", state.cfg.web_public_url, token);
    let locale = EmailLocale::from_str_or_en(&claims.locale);
    let (subject, body_text) =
        render_invite(locale, &active.name, &claims.email, &link).map_err(internal)?;
    state
        .email
        .send(OutboundEmail {
            to_addr: email.clone(),
            to_name: None,
            subject,
            text_body: body_text,
            html_body: None,
        })
        .await
        .map_err(internal)?;

    audit::record(
        &state.audit,
        fid,
        claims.user_id,
        "invite",
        "membership",
        Some(invite_id),
        serde_json::json!({
            "email": email,
            "role": body.role,
            "person_id": body.person_id,
        }),
    )
    .await;
    Ok(ApiResponse::ok(InviteRes { status: "sent" }))
}

// ---------------------------------------------------------------------------
// GET /families/{id}/invites
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/families/{id}/invites",
    operation_id = "invites_list_pending",
    params(
        ("id" = Uuid, Path, description = "Family id (must be a family the caller belongs to)"),
    ),
    responses(
        (status = 200, description = "Pending invites for this family", body = InvitesListResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Insufficient role (admin / owner required)"),
    ),
    security(("cookie_access" = [])),
    tag = "families",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[get("/families/{id}/invites")]
pub async fn list_invites(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<ApiResponse<InvitesList>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let family_id = path.into_inner();
    let active = resolve_membership(&claims, family_id)?;
    crate::auth::require_role(&active, Role::Admin)?;

    let invites = state
        .invites
        .list_pending_for_family(FamilyId::from_uuid(family_id))
        .await
        .map_err(internal)?;
    let data = invites
        .into_iter()
        .map(|i| InviteDto {
            id: i.id,
            email: i.email,
            role: i.invited_role,
            person_id: i.person_id,
            expires_at: i.expires_at,
            invited_by: i.invited_by.into_uuid(),
        })
        .collect();
    Ok(ApiResponse::ok(InvitesList { data }))
}

// ---------------------------------------------------------------------------
// DELETE /families/{id}/invites/{invite_id}
// ---------------------------------------------------------------------------

#[utoipa::path(
    delete,
    path = "/api/v1/families/{id}/invites/{invite_id}",
    operation_id = "invites_cancel",
    params(
        ("id" = Uuid, Path, description = "Family id (must be a family the caller belongs to)"),
        ("invite_id" = Uuid, Path, description = "Pending invite id to cancel"),
    ),
    responses(
        (status = 200, description = "Invite cancelled", body = NullResponseBody, content_type = "application/json"),
        (status = 401, description = "No session"),
        (status = 403, description = "Insufficient role"),
        (status = 404, description = "Invite not pending or already accepted"),
    ),
    security(("cookie_access" = [])),
    tag = "families",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[delete("/families/{id}/invites/{invite_id}")]
pub async fn cancel_invite(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<ApiResponse<serde_json::Value>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let (family_uuid, invite_id) = path.into_inner();
    let active = resolve_membership(&claims, family_uuid)?;
    crate::auth::require_role(&active, Role::Admin)?;

    let family_id = FamilyId::from_uuid(family_uuid);
    state.invites.cancel(invite_id, family_id).await.map_err(|e| match e {
        my_family_domain::InviteRepoError::NotFoundOrAccepted => {
            ApiError::PersonNotFound { id: Some(invite_id) }
        }
        other => ApiError::Internal(anyhow::anyhow!(other.to_string())),
    })?;
    audit::record(
        &state.audit,
        family_id,
        claims.user_id,
        "cancel",
        "invite",
        Some(invite_id),
        serde_json::json!({}),
    )
    .await;
    Ok(ApiResponse::ok(serde_json::Value::Null))
}

// ---------------------------------------------------------------------------
// Helper used by `invites.rs` to build the `FamilyView` field of `AcceptRes`.
// Lives here to keep the construction local to the type.
// ---------------------------------------------------------------------------

#[must_use]
pub(crate) fn family_view_from_claims(
    family_id: Uuid,
    fams: &[FamilyClaim],
    fallback_role: Role,
) -> FamilyView {
    let (name, role) = fams
        .iter()
        .find(|f| f.id == family_id)
        .map_or_else(|| (String::new(), fallback_role), |f| (f.name.clone(), f.role));
    FamilyView { id: family_id, name, role }
}

// ---------------------------------------------------------------------------
// Tests — pure-logic helpers; HTTP integration tests land in Task 8.
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::future_not_send
)]
mod tests {
    use my_family_domain::{FamilyId, Role, UserId};
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
    fn family_view_from_claims_finds_match() {
        let fid = Uuid::new_v4();
        let view = family_view_from_claims(
            fid,
            &[FamilyClaim { id: fid, name: "Müller".into(), role: Role::Admin }],
            Role::User,
        );
        assert_eq!(view.id, fid);
        assert_eq!(view.name, "Müller");
        assert_eq!(view.role, Role::Admin);
    }

    #[test]
    fn family_view_from_claims_falls_back_on_miss() {
        let view = family_view_from_claims(Uuid::new_v4(), &[], Role::User);
        assert_eq!(view.name, "");
        assert_eq!(view.role, Role::User);
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
        })
        .unwrap();
        // The Role serializes by default with derive(Serialize) — we just check
        // the field is present.
        assert_eq!(v["name"], "Müller");
        assert!(v.get("role").is_some());
    }
}
