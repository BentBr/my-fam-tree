//! `/auth/*` endpoints — magic-link request, consume, refresh, logout, me.
//!
//! These five handlers together implement the entire passwordless flow:
//!
//! - `POST /auth/magic-link` — issue a single-use opaque token, store its
//!   sha256, and email a link. Rate-limited per email-address.
//! - `POST /auth/consume`    — exchange a magic-link token for an access JWT
//!   + a refresh cookie. Marks the user verified.
//! - `POST /auth/refresh`    — rotate the refresh cookie and mint a fresh
//!   access JWT; rejects expired or absolute-deadline-exceeded tokens.
//! - `POST /auth/logout`     — revoke the refresh row (if any) and instruct
//!   the browser to drop both cookies.
//! - `GET  /auth/me`         — echo the verified JWT claims so the FE can
//!   bootstrap without a second round-trip.
//!
//! All cookie-setting handlers return `HttpResponse` directly so we can call
//! `add_cookie` on the response builder; `/auth/me` uses the standard
//! `ApiResponse<T>` envelope because it never mutates session state.

use std::time::Duration as StdDuration;

use actix_web::{HttpRequest, HttpResponse, post, web};
use chrono::{Duration, Utc};
use my_fam_tree_domain::{Locale, MagicLinkRepoError};
use my_fam_tree_email::{Locale as EmailLocale, render_magic_link};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::{FamilyClaim, generate_opaque_token, hash_token};
use crate::cookies::{
    ACCESS_COOKIE, REFRESH_COOKIE, REFRESH_COOKIE_PATH, access_cookie, refresh_cookie, revoked,
};
use crate::services::auth_service::{issue_access_token_for, mint_magic_link_url};
use crate::validation::{email_invalid, looks_like_email};
use crate::{ApiError, ApiResponse, AppState, response_body};

/// Per-IP sliding-window rate cap shared by the token-validation endpoints
/// (`/auth/consume`, `/auth/refresh`, `/invites/accept`,
/// `/owner-transfer/confirm`). 256-bit opaque tokens make brute force
/// infeasible — this guard only exists to keep a runaway client from
/// drowning the DB in `find-by-hash` lookups.
///
/// `prefix` is namespaced per endpoint (e.g. `"consume:ip"`) so a busy
/// `/auth/refresh` doesn't eat the `consume` budget. `max_per_hour` is
/// expected to be generous (~120).
#[allow(clippy::future_not_send, reason = "actix HttpRequest is !Send by design")]
pub(crate) async fn rate_limit_ip(
    state: &web::Data<AppState>,
    req: &HttpRequest,
    prefix: &str,
    max_per_hour: u32,
) -> Result<(), ApiError> {
    let ip = req.connection_info().realip_remote_addr().unwrap_or("unknown").to_string();
    let decision = state
        .rate_limiter
        .check(&format!("{prefix}:{ip}"), max_per_hour, StdDuration::from_hours(1))
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;
    if !decision.allowed {
        return Err(ApiError::RateLimited { retry_after_secs: decision.retry_after_seconds });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Request / response DTOs.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MagicLinkReq {
    pub email: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MagicLinkRes {
    pub status: &'static str,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConsumeReq {
    pub token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConsumeRes {
    pub user_id: uuid::Uuid,
    pub email: String,
    pub locale: String,
    pub families: Vec<FamilyClaim>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LogoutRes {
    pub status: &'static str,
}

response_body!(pub MagicLinkResponseBody, MagicLinkRes);
response_body!(pub ConsumeResponseBody, ConsumeRes);
response_body!(pub LogoutResponseBody, LogoutRes);
// `/auth/me` returns the same shape as `/auth/consume` — a `ConsumeRes` body.
// Schemas in OpenAPI need distinct names per endpoint for client codegen to
// name the response types meaningfully, so we declare a dedicated wrapper.
response_body!(pub MeResponseBody, ConsumeRes);

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

fn seconds_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

const fn build_consume_response(
    user_id: uuid::Uuid,
    email: String,
    locale: String,
    families: Vec<FamilyClaim>,
) -> ConsumeRes {
    ConsumeRes { user_id, email, locale, families }
}

// ---------------------------------------------------------------------------
// POST /auth/magic-link
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/auth/magic-link",
    request_body = MagicLinkReq,
    responses(
        (status = 200, description = "Magic link sent", body = MagicLinkResponseBody),
        (status = 422, description = "Invalid email"),
        (status = 429, description = "Rate limited"),
    ),
    tag = "auth",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/auth/magic-link")]
pub async fn magic_link(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<MagicLinkReq>,
) -> Result<ApiResponse<MagicLinkRes>, ApiError> {
    let email = body.email.trim().to_lowercase();
    if !looks_like_email(&email) {
        return Err(email_invalid("/email"));
    }

    // Per-email sliding-window rate limit. Stops a single address from
    // being spammed (or self-spammed by an automation gone wrong).
    let decision = state
        .rate_limiter
        .check(
            &format!("ml:email:{email}"),
            state.cfg.magic_link.rate_per_email_per_hour,
            StdDuration::from_hours(1),
        )
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;
    if !decision.allowed {
        return Err(ApiError::RateLimited { retry_after_secs: decision.retry_after_seconds });
    }

    // Per-IP sliding-window rate limit (security audit MEDIUM). The config
    // already exposes `magic_link.rate_per_ip_per_hour` but the handler
    // never applied it — letting an attacker iterate over email addresses
    // from a single IP to drain the SMTP outbox / spam arbitrary mailboxes.
    // `realip_remote_addr()` returns the peer addr (or the leading X-Forwarded-For
    // entry when a trusted proxy sets it); `"unknown"` is a safe fallback that
    // keeps the per-IP bucket usable even when actix can't resolve the peer.
    let ip = req.connection_info().realip_remote_addr().unwrap_or("unknown").to_string();
    let ip_decision = state
        .rate_limiter
        .check(
            &format!("ml:ip:{ip}"),
            state.cfg.magic_link.rate_per_ip_per_hour,
            StdDuration::from_hours(1),
        )
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;
    if !ip_decision.allowed {
        return Err(ApiError::RateLimited { retry_after_secs: ip_decision.retry_after_seconds });
    }

    // Lookup or create the user. We use the user's locale (or default En) to
    // pick the email template; missing-display-name is handled downstream.
    let user = match state
        .users
        .find_by_email(&email)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?
    {
        Some(u) => u,
        None => state
            .users
            .create(&email, Locale::En)
            .await
            .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?,
    };

    // Issue and persist a single-use token; the helper returns the full
    // consume URL (only the hash hits the DB).
    let link = mint_magic_link_url(
        &state.magic_links,
        user.id,
        &user.email,
        &state.cfg.web.public_url,
        state.cfg.magic_link.ttl_seconds,
    )
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;

    // Render the email + enqueue it into the durable outbox. Postgres is
    // the source of truth, so a Redis flush can't lose mail and SMTP
    // slowness no longer blocks this request thread — the worker drains
    // the outbox out-of-band via SMTP with retry/backoff.
    let locale = EmailLocale::from_str_or_en(user.locale.as_str());
    let (subject, text_body) = render_magic_link(locale, &link)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;
    state
        .outbox
        .enqueue(&my_fam_tree_domain::EmailOutboxInsert {
            kind: my_fam_tree_domain::EmailOutboxKind::MAGIC_LINK.to_string(),
            to_addr: user.email.clone(),
            subject,
            text_body,
            html_body: None,
        })
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;

    Ok(ApiResponse::ok(MagicLinkRes { status: "sent" }))
}

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

    // Rotate a fresh opaque refresh token with both rolling + absolute TTLs.
    let (refresh_token, refresh_hash) = generate_opaque_token();
    let now = Utc::now();
    state
        .refresh_tokens
        .create(
            user.id,
            &refresh_hash,
            None,
            None,
            None,
            now + Duration::seconds(seconds_i64(state.cfg.jwt.refresh_ttl_seconds)),
            now + Duration::seconds(seconds_i64(state.cfg.jwt.refresh_absolute_ttl_seconds)),
        )
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;

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

    let record = state
        .refresh_tokens
        .find_active_by_hash(&old_hash)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?
        .ok_or(ApiError::RefreshInvalid)?;

    let now = Utc::now();
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

// ---------------------------------------------------------------------------
// POST /auth/logout
// ---------------------------------------------------------------------------

/// POST `/auth/logout` — clear the session.
///
/// PUBLIC by design: this endpoint is mounted OUTSIDE the required-auth
/// scope (see `routes::mod`). The FE needs to call it AFTER a session
/// has already collapsed server-side (e.g., when the refresh token is
/// revoked or the access cookie expired and the refresh round-trip
/// failed) — at which point an auth-gated logout would 401 and the
/// `HttpOnly` cookies would linger in the browser indefinitely.
///
/// The handler is idempotent:
///   - if the refresh cookie is present, best-effort revoke the
///     matching row in the DB;
///   - always emit `Set-Cookie max-age=0` for both cookies so the
///     browser drops them;
///   - return 200 regardless.
///
/// Reveals no state — the response body is the same fixed
/// `{ status: "logged out" }` for every caller, so making the endpoint
/// public exposes nothing an unauthenticated probe could not already
/// observe (the Set-Cookie clearing headers don't carry session info).
//
// The doc above intentionally runs long — the security rationale for
// "why public" lives at the call site so future reviewers can audit
// without chasing references. Localised allow keeps the nursery lint
// in place for other items where shorter docs are still preferable.
#[allow(clippy::too_long_first_doc_paragraph)]
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    responses(
        (status = 200, description = "Logged out (or no session to begin with)", body = LogoutResponseBody),
    ),
    tag = "auth",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/auth/logout")]
pub async fn logout(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    if let Some(c) = req.cookie(REFRESH_COOKIE) {
        let hash = hash_token(c.value());
        // Best-effort: a refresh-row revoke failure must not block logout.
        let _ = state.refresh_tokens.revoke_by_hash(&hash).await;
    }
    let mut resp = HttpResponse::Ok().json(ApiResponse::ok(LogoutRes { status: "logged out" }));
    let _ = resp.add_cookie(&revoked(&state.cfg, ACCESS_COOKIE, "/"));
    let _ = resp.add_cookie(&revoked(&state.cfg, REFRESH_COOKIE, REFRESH_COOKIE_PATH));
    Ok(resp)
}

// ---------------------------------------------------------------------------
// GET /auth/me
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    responses(
        (status = 200, description = "Current session claims", body = MeResponseBody),
        (status = 401, description = "No session"),
    ),
    security(("cookie_access" = [])),
    tag = "auth",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[actix_web::get("/auth/me")]
pub async fn me(req: HttpRequest) -> Result<ApiResponse<ConsumeRes>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let families = claims
        .all_families
        .into_iter()
        .map(|f| FamilyClaim { id: f.id.into_uuid(), name: f.name, role: f.role })
        .collect();
    Ok(ApiResponse::ok(build_consume_response(
        claims.user_id.into_uuid(),
        claims.email,
        claims.locale,
        families,
    )))
}

// ---------------------------------------------------------------------------
// Tests — pure-logic helpers; HTTP integration tests land in Task 8.
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::future_not_send,
    clippy::indexing_slicing,
    clippy::panic
)]
mod tests {
    use my_fam_tree_domain::{MagicLinkPurpose, Role};
    use uuid::Uuid;

    use super::*;

    #[test]
    fn seconds_i64_clamps_overflow_to_max() {
        assert_eq!(seconds_i64(0), 0);
        assert_eq!(seconds_i64(900), 900);
        assert_eq!(seconds_i64(u64::MAX), i64::MAX);
    }

    #[test]
    fn magic_link_purpose_login_serialises_to_login() {
        assert_eq!(MagicLinkPurpose::Login.as_db(), "login");
    }

    #[test]
    fn build_consume_response_carries_all_fields() {
        let uid = Uuid::new_v4();
        let fid = Uuid::new_v4();
        let res = build_consume_response(
            uid,
            "a@b.c".into(),
            "de".into(),
            vec![FamilyClaim { id: fid, name: "Müller".into(), role: Role::Owner }],
        );
        assert_eq!(res.user_id, uid);
        assert_eq!(res.email, "a@b.c");
        assert_eq!(res.locale, "de");
        assert_eq!(res.families.len(), 1);
        assert_eq!(res.families[0].id, fid);
        assert_eq!(res.families[0].role, Role::Owner);
    }

    #[test]
    fn magic_link_res_serialises_with_status_field() {
        let body = MagicLinkRes { status: "sent" };
        let v = serde_json::to_value(&body).unwrap();
        assert_eq!(v["status"], "sent");
    }

    #[test]
    fn logout_res_serialises_with_status_field() {
        let body = LogoutRes { status: "logged out" };
        let v = serde_json::to_value(&body).unwrap();
        assert_eq!(v["status"], "logged out");
    }

    #[test]
    fn consume_req_deserialises_from_token_field() {
        let req: ConsumeReq = serde_json::from_value(serde_json::json!({"token": "abc"})).unwrap();
        assert_eq!(req.token, "abc");
    }

    #[test]
    fn magic_link_req_deserialises_from_email_field() {
        let req: MagicLinkReq =
            serde_json::from_value(serde_json::json!({"email": "a@b.c"})).unwrap();
        assert_eq!(req.email, "a@b.c");
    }
}
