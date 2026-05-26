//! `/invites/accept` + `/families/{id}/invites` GET/DELETE endpoints.
//!
//! `accept` lives here for historical reasons (it pre-dates Phase D's
//! list / cancel endpoints). The caller MUST already be authenticated —
//! every route in this file lives under [`AuthMiddleware::required`]. We
//! atomically claim the invite, verify the signed-in email matches the
//! address the invite was sent to, insert a membership row at the
//! invited role, and reissue the access cookie so the new family is
//! immediately reflected in the JWT.
//!
//! The "email mismatch" check is intentionally surfaced as a `Validation`
//! error (not `InviteExpired`/`MagicLinkInvalid`) so the FE can render an
//! actionable hint: the user signed in with the wrong account.
//!
//! `list_invites` + `cancel_invite` are Phase-D admin-only surfaces. Both
//! live in this module (next to `accept`) to keep all invite handlers in
//! one place; `families.rs` only owns the `POST` half because the existing
//! `families::invite` handler ships from there.

use actix_web::{HttpRequest, HttpResponse, delete, get, post, web};
use chrono::{DateTime, Utc};
use my_family_domain::{FamilyId, InviteRepoError, PersonId, Role};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::{FamilyClaim, hash_token};
use crate::cookies::access_cookie;
use crate::response::NullResponseBody;
use crate::routes::families::FamilyView;
use crate::services::audit;
use crate::services::auth_service::issue_access_token_for;
use crate::validation::invite_email_mismatch;
use crate::{ApiError, ApiResponse, AppState, response_body};

/// Build a `FamilyView` from a list of `FamilyClaim`s (the JWT-mirrored
/// memberships returned by `issue_access_token_for`). Used by the accept
/// route to project the newly-joined family without an extra DB round-trip.
#[must_use]
fn family_view_from_claims(
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
// Request / response DTOs.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
pub struct AcceptReq {
    pub token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AcceptRes {
    pub family: FamilyView,
    pub claims: super::auth::ConsumeRes,
}

/// Wire DTO returned by `GET /families/{id}/invites`. Mirrors the
/// persisted `Invite` minus the token hash (which never leaves the
/// database).
#[derive(Debug, Serialize, ToSchema)]
pub struct InviteDto {
    pub id: Uuid,
    pub email: String,
    pub role: Role,
    pub person_id: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
    pub invited_by: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InvitesList {
    pub data: Vec<InviteDto>,
}

response_body!(pub AcceptResponseBody, AcceptRes);
response_body!(pub InvitesListResponseBody, InvitesList);

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

// ---------------------------------------------------------------------------
// POST /invites/accept
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/invites/accept",
    request_body = AcceptReq,
    responses(
        (status = 200, description = "Invite accepted, membership inserted, session refreshed", body = AcceptResponseBody),
        (status = 401, description = "No session or invite token invalid"),
        (status = 410, description = "Invite expired"),
        (status = 422, description = "Invite belongs to a different email"),
    ),
    security(("cookie_access" = [])),
    tag = "invites",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/invites/accept")]
pub async fn accept(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<AcceptReq>,
) -> Result<HttpResponse, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let hash = hash_token(body.token.trim());
    let invite = state.invites.accept(&hash, Utc::now()).await.map_err(|e| match e {
        InviteRepoError::Expired => ApiError::InviteExpired,
        InviteRepoError::NotFoundOrAccepted => ApiError::MagicLinkInvalid,
        InviteRepoError::Db(s) => ApiError::Internal(anyhow::anyhow!(s)),
    })?;

    if !invite.email.eq_ignore_ascii_case(&claims.email) {
        return Err(invite_email_mismatch("/token"));
    }

    // Audit the email-match verification BEFORE writing membership so the
    // log keeps an event even if the membership insert below races against
    // a concurrent admin revoke. metadata.person_id (if set) lets the audit
    // table link the row back to the person via the existing CASE.
    audit::record(
        &state.audit,
        invite.family_id,
        claims.user_id,
        "verify",
        "invite",
        Some(invite.id),
        serde_json::json!({
            "email": invite.email,
            "person_id": invite.person_id,
        }),
    )
    .await;

    state
        .memberships
        .insert(invite.family_id, claims.user_id, invite.invited_role)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;
    audit::record(
        &state.audit,
        invite.family_id,
        claims.user_id,
        "accept_invite",
        "membership",
        None,
        serde_json::json!({
            "user_id": claims.user_id.into_uuid(),
            "role": invite.invited_role,
        }),
    )
    .await;

    // Wire `persons.linked_user_id` so the recipient becomes the person
    // they were invited as. Best-effort: if the persons row was deleted
    // between invite-issue and accept (ON DELETE SET NULL kept the invite
    // alive), we surface the resulting `NotFound` as `Internal` — the
    // membership already exists, so the worst case is an unlinked-but-
    // joined user that the admin can fix afterwards.
    if let Some(person_id) = invite.person_id {
        state
            .persons
            .set_linked_user_id(
                invite.family_id,
                PersonId::from_uuid(person_id),
                Some(claims.user_id),
            )
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;
    }

    let user = state
        .users
        .find_by_id(claims.user_id)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?
        .ok_or(ApiError::Unauthenticated)?;
    let (access, fams) = issue_access_token_for(&state.jwt_issuer, &state.memberships, &user)
        .await
        .map_err(ApiError::Internal)?;

    let family = family_view_from_claims(invite.family_id.into_uuid(), &fams, invite.invited_role);
    let payload = AcceptRes {
        family,
        claims: super::auth::ConsumeRes {
            user_id: user.id.into_uuid(),
            email: user.email.clone(),
            locale: user.locale.as_str().to_string(),
            families: fams,
        },
    };

    let mut resp = HttpResponse::Ok().json(ApiResponse::ok(payload));
    let _ = resp.add_cookie(&access_cookie(&state.cfg, access));
    Ok(resp)
}

// ---------------------------------------------------------------------------
// GET /families/{id}/invites — admin/owner only.
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
    tag = "invites",
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
    let active = super::families::resolve_membership(&claims, family_id)?;
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
// DELETE /families/{id}/invites/{invite_id} — admin/owner only.
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
    tag = "invites",
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
    let active = super::families::resolve_membership(&claims, family_uuid)?;
    crate::auth::require_role(&active, Role::Admin)?;

    let family_id = FamilyId::from_uuid(family_uuid);
    state.invites.cancel(invite_id, family_id).await.map_err(|e| match e {
        InviteRepoError::NotFoundOrAccepted => ApiError::PersonNotFound { id: Some(invite_id) },
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
// Tests — pure-logic DTO checks; HTTP integration tests land in Task 8.
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn accept_req_deserialises_from_token_field() {
        let r: AcceptReq = serde_json::from_value(serde_json::json!({"token": "abc"})).unwrap();
        assert_eq!(r.token, "abc");
    }
}
