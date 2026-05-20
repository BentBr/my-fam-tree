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
use my_family_domain::{Locale, MagicLinkPurpose, MagicLinkRepoError};
use my_family_email::{Locale as EmailLocale, OutboundEmail, render_magic_link};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::{FamilyClaim, generate_opaque_token, hash_token};
use crate::cookies::{
    ACCESS_COOKIE, REFRESH_COOKIE, REFRESH_COOKIE_PATH, access_cookie, refresh_cookie, revoked,
};
use crate::services::auth_service::issue_access_token_for;
use crate::{ApiError, ApiResponse, AppState, FieldViolation, response_body};

// ---------------------------------------------------------------------------
// Request / response DTOs.
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
pub struct MagicLinkReq {
    pub email: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MagicLinkRes {
    pub status: &'static str,
}

#[derive(Debug, Deserialize, ToSchema)]
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

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

/// Lightweight email syntax check. NOT RFC 5322 — full validation belongs at
/// the SMTP send layer. Here we just reject typos so the rate-limiter key
/// isn't polluted with garbage and so the user sees the "must be an email"
/// error before we issue a magic link.
///
/// Rules enforced:
///  - Exactly one `@`.
///  - Local part: 1–64 ASCII chars from `[A-Za-z0-9._%+-]`; no leading/trailing
///    `.` and no consecutive `..`.
///  - Domain: 1+ ASCII labels separated by `.`; each label 1–63 chars from
///    `[A-Za-z0-9-]`, no leading/trailing `-`.
///  - TLD: ≥ 2 ASCII letters (no digits, no hyphens). This rejects `a@b.c`
///    and `a@b.c1` but accepts `a@b.co`, `a@b.museum`, etc.
fn looks_like_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    if local.is_empty() || local.len() > 64 || domain.is_empty() {
        return false;
    }
    if domain.contains('@') {
        return false;
    }
    if !is_valid_local_part(local) {
        return false;
    }
    is_valid_domain(domain)
}

fn is_valid_local_part(local: &str) -> bool {
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return false;
    }
    local.bytes().all(|b| b.is_ascii_alphanumeric() || b".!#$%&'*+/=?^_`{|}~.-".contains(&b))
}

fn is_valid_domain(domain: &str) -> bool {
    let labels: Vec<&str> = domain.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    if !labels.iter().all(|label| is_valid_domain_label(label)) {
        return false;
    }
    // TLD must be ≥ 2 ASCII letters.
    let Some(tld) = labels.last() else {
        return false;
    };
    tld.len() >= 2 && tld.bytes().all(|b| b.is_ascii_alphabetic())
}

fn is_valid_domain_label(label: &str) -> bool {
    if label.is_empty() || label.len() > 63 {
        return false;
    }
    if label.starts_with('-') || label.ends_with('-') {
        return false;
    }
    label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

fn email_validation_error() -> ApiError {
    ApiError::Validation(vec![FieldViolation {
        path: "/email".into(),
        code: "validation.email_invalid".into(),
        message: "must be an email".into(),
    }])
}

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
    body: web::Json<MagicLinkReq>,
) -> Result<ApiResponse<MagicLinkRes>, ApiError> {
    let email = body.email.trim().to_lowercase();
    if !looks_like_email(&email) {
        return Err(email_validation_error());
    }

    // Per-email sliding-window rate limit.
    let decision = state
        .rate_limiter
        .check(
            &format!("ml:email:{email}"),
            state.cfg.magic_link_rate_per_email_per_hour,
            StdDuration::from_hours(1),
        )
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;
    if !decision.allowed {
        return Err(ApiError::RateLimited { retry_after_secs: decision.retry_after_seconds });
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

    // Issue and persist a single-use token (we store only the hash).
    let (token, hash) = generate_opaque_token();
    state
        .magic_links
        .create(
            Some(user.id),
            &user.email,
            &hash,
            MagicLinkPurpose::Login,
            Utc::now() + Duration::seconds(seconds_i64(state.cfg.magic_link_ttl_seconds)),
        )
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;

    // Render + send the email.
    let link = format!("{}/auth/consume?token={}", state.cfg.web_public_url, token);
    let locale = EmailLocale::from_str_or_en(user.locale.as_str());
    let (subject, text_body) = render_magic_link(locale, &link)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;
    state
        .email
        .send(OutboundEmail {
            to_addr: user.email.clone(),
            to_name: None,
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
    body: web::Json<ConsumeReq>,
) -> Result<HttpResponse, ApiError> {
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
            now + Duration::seconds(seconds_i64(state.cfg.jwt_refresh_ttl_seconds)),
            now + Duration::seconds(seconds_i64(state.cfg.jwt_refresh_absolute_ttl_seconds)),
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
            now + Duration::seconds(seconds_i64(state.cfg.jwt_refresh_ttl_seconds)),
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

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    responses(
        (status = 200, description = "Logged out", body = LogoutResponseBody),
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
    let _ = resp.add_cookie(&revoked(ACCESS_COOKIE, "/"));
    let _ = resp.add_cookie(&revoked(REFRESH_COOKIE, REFRESH_COOKIE_PATH));
    Ok(resp)
}

// ---------------------------------------------------------------------------
// GET /auth/me
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    responses(
        (status = 200, description = "Current session claims", body = ConsumeResponseBody),
        (status = 401, description = "No session"),
    ),
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
    use my_family_domain::Role;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn looks_like_email_accepts_well_formed_addresses() {
        assert!(looks_like_email("a@b.co"));
        assert!(looks_like_email("user@example.org"));
        assert!(looks_like_email("user.name+tag@sub.example.org"));
        assert!(looks_like_email("user_123@example.co.uk"));
    }

    #[test]
    fn looks_like_email_rejects_malformed_addresses() {
        // No @ / empty / multiple @
        assert!(!looks_like_email(""));
        assert!(!looks_like_email("nope"));
        assert!(!looks_like_email("@example.com"));
        assert!(!looks_like_email("user@"));
        assert!(!looks_like_email("a@@b.co"));
        assert!(!looks_like_email("a@b@c.co"));
        // No TLD or single-letter TLD
        assert!(!looks_like_email("a@b"));
        assert!(!looks_like_email("a@b.c"));
        // TLD with digits or hyphens
        assert!(!looks_like_email("a@b.c1"));
        assert!(!looks_like_email("a@b.c-d"));
        // Leading/trailing dot or consecutive dots in local part
        assert!(!looks_like_email(".user@example.com"));
        assert!(!looks_like_email("user.@example.com"));
        assert!(!looks_like_email("us..er@example.com"));
        // Domain label leading/trailing hyphen
        assert!(!looks_like_email("user@-example.com"));
        assert!(!looks_like_email("user@example-.com"));
        // Non-ASCII in local/domain
        assert!(!looks_like_email("üser@example.com"));
        assert!(!looks_like_email("user@exämple.com"));
        // Whitespace
        assert!(!looks_like_email("us er@example.com"));
        assert!(!looks_like_email("user@example .com"));
    }

    #[test]
    fn email_validation_error_uses_stable_path_and_code() {
        let err = email_validation_error();
        match err {
            ApiError::Validation(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0].path, "/email");
                assert_eq!(v[0].code, "validation.email_invalid");
            }
            _ => panic!("expected Validation"),
        }
    }

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
