//! `/invites/accept` + `/families/{id}/invites` GET/DELETE endpoints.
//!
//! `accept` is intentionally reachable by *anonymous* callers — the
//! invite token (a 32-byte URL-safe blob, hashed and single-use server-
//! side) is treated as the authentication factor. If the request
//! arrives without a session, we find-or-create the user keyed on
//! `invite.email`, issue an access cookie, and proceed. If a session
//! IS present, we keep the existing behaviour: validate the session
//! email matches `invite.email`, surface a 422 `invite_email_mismatch`
//! otherwise (the FE renders an actionable "sign out first" hint).
//!
//! `list_invites` + `cancel_invite` are Phase-D admin-only surfaces;
//! both still live under [`AuthMiddleware::required`]. They live in
//! this module (next to `accept`) to keep all invite handlers in one
//! place; `families.rs` only owns the `POST` half because the existing
//! `families::invite` handler ships from there.

use actix_web::{HttpRequest, HttpResponse, delete, get, post, web};
use chrono::{DateTime, Utc};
use my_fam_tree_domain::{FamilyId, InviteRepoError, Locale, PersonId, Role, UserId};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::{FamilyClaim, hash_token};
use crate::cookies::{ACCESS_COOKIE, access_cookie, refresh_cookie};
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
    FamilyView { id: family_id, name, role, created_at: None }
}

// ---------------------------------------------------------------------------
// Request / response DTOs.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
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
        (status = 200, description = "Invite accepted, membership inserted, session issued", body = AcceptResponseBody),
        (status = 401, description = "Invite token invalid"),
        (status = 410, description = "Invite expired"),
        (status = 422, description = "Existing session belongs to a different email"),
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
    // Per-IP rate cap (security audit INFO). See `auth::rate_limit_ip` for why.
    crate::routes::auth::rate_limit_ip(&state, &req, "invite-accept:ip", 120).await?;

    // The session (if any) is optional. If an active session is bound
    // to a different email than the invite, we reject with 422 so the
    // user can sign out first. If no session, the invite token itself
    // is the auth factor — we resolve the user by `invite.email`.
    //
    // We extract claims manually (rather than through `user_claims(&req)`)
    // because this route is mounted OUTSIDE the AuthMiddleware-wrapped
    // scope: actix's empty-path sibling-scope resolution stops at the
    // first scope and would 404 anonymous callers if the middleware were
    // applied.
    let existing_claims_email_and_id: Option<(String, my_fam_tree_domain::UserId)> = req
        .cookie(ACCESS_COOKIE)
        .and_then(|c| state.jwt_issuer.verify(c.value()).ok())
        .map(|jwt| (jwt.email.clone(), my_fam_tree_domain::UserId::from_uuid(jwt.sub)));

    let hash = hash_token(body.token.trim());
    let invite = state.invites.accept(&hash, Utc::now()).await.map_err(|e| match e {
        InviteRepoError::Expired => ApiError::InviteExpired,
        InviteRepoError::NotFoundOrAccepted => ApiError::MagicLinkInvalid,
        InviteRepoError::Db(s) => ApiError::Internal(anyhow::anyhow!(s)),
    })?;

    // Resolve the acting user: either the active session's user (if the
    // email matches the invite) or — when anonymous — the user keyed on
    // `invite.email`, creating the row if it doesn't exist yet.
    let user_id: UserId = if let Some((email, uid)) = existing_claims_email_and_id.as_ref() {
        if !invite.email.eq_ignore_ascii_case(email) {
            return Err(invite_email_mismatch("/token"));
        }
        *uid
    } else {
        match state.users.find_by_email(&invite.email).await.map_err(internal)? {
            Some(u) => u.id,
            None => state.users.create(&invite.email, Locale::En).await.map_err(internal)?.id,
        }
    };

    // Audit the verification BEFORE writing membership so the log keeps
    // an event even if the membership insert below races. metadata.person_id
    // (if set) lets the audit table link back to the person via the CASE.
    audit::record(
        &state.audit,
        invite.family_id,
        user_id,
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
        .insert(invite.family_id, user_id, invite.invited_role)
        .await
        .map_err(internal)?;
    audit::record(
        &state.audit,
        invite.family_id,
        user_id,
        "accept_invite",
        "membership",
        None,
        serde_json::json!({
            "user_id": user_id.into_uuid(),
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
            .set_linked_user_id(invite.family_id, PersonId::from_uuid(person_id), Some(user_id))
            .await
            .map_err(internal)?;
    }

    let user = state
        .users
        .find_by_id(user_id)
        .await
        .map_err(internal)?
        .ok_or(ApiError::Unauthenticated)?;
    let (access, fams) = issue_access_token_for(&state.jwt_issuer, &state.memberships, &user)
        .await
        .map_err(ApiError::Internal)?;

    // Mint + persist a refresh token alongside the access cookie via the
    // shared service helper. Without this row (and the matching cookie),
    // invite-accept only set the SHORT-lived access cookie (~15 min
    // TTL) and the recipient looked logged out the next time they came
    // back — they had to request a fresh magic link. Same helper as
    // `/auth/consume` so the two sign-in paths stay in lockstep.
    let refresh_token = crate::services::auth_service::mint_refresh_token_for(
        &state.refresh_tokens,
        &state.cfg.jwt,
        &user,
    )
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
    let _ = resp.add_cookie(&refresh_cookie(&state.cfg, refresh_token));
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
    crate::auth::require_db_role(&state, claims.user_id, active.id, Role::Admin).await?;

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
    crate::auth::require_db_role(&state, claims.user_id, active.id, Role::Admin).await?;

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
