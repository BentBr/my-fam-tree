//! `/invites/accept` endpoint.
//!
//! The caller MUST already be authenticated (the route lives under
//! [`AuthMiddleware::required`]). We atomically claim the invite, verify the
//! signed-in email matches the address the invite was sent to, insert a
//! membership row at the invited role, and reissue the access cookie so the
//! new family is immediately reflected in the JWT.
//!
//! The "email mismatch" check is intentionally surfaced as a `Validation`
//! error (not `InviteExpired`/`MagicLinkInvalid`) so the FE can render an
//! actionable hint: the user signed in with the wrong account.

use actix_web::{HttpRequest, HttpResponse, post, web};
use chrono::Utc;
use my_family_domain::InviteRepoError;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::hash_token;
use crate::cookies::access_cookie;
use crate::routes::families::{FamilyView, family_view_from_claims};
use crate::services::auth_service::issue_access_token_for;
use crate::validation::invite_email_mismatch;
use crate::{ApiError, ApiResponse, AppState, response_body};

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

response_body!(pub AcceptResponseBody, AcceptRes);

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

    state
        .memberships
        .insert(invite.family_id, claims.user_id, invite.invited_role)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;

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
