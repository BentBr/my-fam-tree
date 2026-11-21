//! `POST /auth/consume` and `POST /auth/refresh` — token exchange +
//! refresh-cookie rotation.

use actix_web::{HttpRequest, HttpResponse, post, web};
use my_fam_tree_domain::MagicLinkRepoError;
use time::{Duration, OffsetDateTime};

use super::{ConsumeReq, ConsumeResponseBody, build_consume_response, rate_limit_ip, seconds_i64};
use crate::auth::{generate_opaque_token, hash_token};
use crate::cookies::{REFRESH_COOKIE, access_cookie, refresh_cookie};
use crate::services::auth_service::issue_access_token_for;
use crate::{ApiError, ApiResponse, AppState};

// ---------------------------------------------------------------------------
// POST /auth/consume
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/auth/consume",
    request_body = ConsumeReq,
    responses(
        (status = 200, description = "Magic link consumed, session established", body = ConsumeResponseBody),
        (status = 401, description = "Magic link invalid or expired"),
    ),
    tag = "auth",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/auth/consume")]
pub async fn consume(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<ConsumeReq>,
) -> Result<HttpResponse, ApiError> {
    // Per-IP rate cap (security audit INFO). 256-bit tokens make brute
    // force infeasible, so this is purely a DB-DoS guard against
    // token-guess storms; bound generously at 120/hour.
    rate_limit_ip(&state, &req, "consume:ip", 120).await?;

    let token = body.token.trim();
    if token.is_empty() {
        return Err(ApiError::MagicLinkInvalid);
    }

    let hash = hash_token(token);
    let record = state.magic_links.consume(&hash).await.map_err(|e| match e {
        MagicLinkRepoError::Expired | MagicLinkRepoError::NotFoundOrConsumed => {
            ApiError::MagicLinkInvalid
        }
        MagicLinkRepoError::Db(s) => ApiError::Internal(anyhow::anyhow!(s)),
    })?;

    let user_id = record.user_id.ok_or(ApiError::MagicLinkInvalid)?;
    let user = state
        .users
        .find_by_id(user_id)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?
        .ok_or(ApiError::MagicLinkInvalid)?;
    state
        .users
        .mark_verified(user.id)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;

    // Mint a fresh access JWT bundling every family the user belongs to.
    let (access, families) =
        issue_access_token_for(&state.jwt_issuer, &state.memberships, &user).await?;

    // Mint + persist a fresh refresh token via the shared service helper
    // so consume + invite-accept stay in lockstep on TTL config + repo
    // semantics.
    let refresh_token = crate::services::auth_service::mint_refresh_token_for(
        &state.refresh_tokens,
        &state.cfg.jwt,
        &user,
    )
    .await
    .map_err(ApiError::Internal)?;

    let response = build_consume_response(
        user.id.into_uuid(),
        user.email.clone(),
        user.locale.as_str().to_string(),
        families,
    );

    let mut resp = HttpResponse::Ok().json(ApiResponse::ok(response));
    let _ = resp.add_cookie(&access_cookie(&state.cfg, access));
    let _ = resp.add_cookie(&refresh_cookie(&state.cfg, refresh_token));
    Ok(resp)
}

// ---------------------------------------------------------------------------
// POST /auth/refresh
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    responses(
        (status = 200, description = "Session refreshed", body = ConsumeResponseBody),
        (status = 401, description = "Refresh token invalid or expired"),
    ),
    tag = "auth",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/auth/refresh")]
pub async fn refresh(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    // Per-IP rate cap (security audit INFO). Same shape as `consume`.
    rate_limit_ip(&state, &req, "refresh:ip", 120).await?;

    let cookie = req.cookie(REFRESH_COOKIE).ok_or(ApiError::RefreshInvalid)?;
    let old_hash = hash_token(cookie.value());

    let Some(record) = state
        .refresh_tokens
        .find_active_by_hash(&old_hash)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?
    else {
        // No active row for this hash. Before rejecting, check whether the
        // hash matches an ALREADY-REVOKED row — that means a previously
        // rotated (and thus single-use-spent) token was re-presented:
        // classic refresh-token reuse / theft. Defensively revoke EVERY
        // session for the owning user so a stolen token can't be chained.
        if let Some(rec) = state
            .refresh_tokens
            .find_any_by_hash(&old_hash)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?
            && rec.revoked_at.is_some()
        {
            let _ = state.refresh_tokens.revoke_all_for_user(rec.user_id).await;
            tracing::warn!(
                user_id = %rec.user_id.into_uuid(),
                "refresh token reuse detected; revoked all sessions for user"
            );
        }
        return Err(ApiError::RefreshInvalid);
    };

    let now = OffsetDateTime::now_utc();
    if record.expires_at < now || record.absolute_expires_at < now {
        return Err(ApiError::RefreshInvalid);
    }

    let user = state
        .users
        .find_by_id(record.user_id)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?
        .ok_or(ApiError::RefreshInvalid)?;

    // Rotate the refresh row: revoke the old, insert a new with the same
    // device label, and renew the rolling deadline (absolute deadline stays).
    let (new_refresh_token, new_hash) = generate_opaque_token();
    state
        .refresh_tokens
        .rotate(
            &old_hash,
            &new_hash,
            now + Duration::seconds(seconds_i64(state.cfg.jwt.refresh_ttl_seconds)),
            record.device_label.as_deref(),
            None,
            None,
        )
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;

    let (access, families) =
        issue_access_token_for(&state.jwt_issuer, &state.memberships, &user).await?;

    let response = build_consume_response(
        user.id.into_uuid(),
        user.email.clone(),
        user.locale.as_str().to_string(),
        families,
    );

    let mut resp = HttpResponse::Ok().json(ApiResponse::ok(response));
    let _ = resp.add_cookie(&access_cookie(&state.cfg, access));
    let _ = resp.add_cookie(&refresh_cookie(&state.cfg, new_refresh_token));
    Ok(resp)
}
