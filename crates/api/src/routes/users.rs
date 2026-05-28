//! `/users/me` — caller-scoped profile read, update, and email-change flow.
//!
//! - `GET   /users/me`                       — fresh DB read of the caller's
//!   profile (the JWT mirror in `auth/me` is for fast bootstrap; this route
//!   is the source of truth).
//! - `PATCH /users/me`                       — patch `display_name` and/or
//!   `locale`. Empty body → 422 `validation.value_required`.
//! - `POST  /users/me/email-change`          — start the change. We email a
//!   single-use link to the user's **current** address; the body of the email
//!   mentions the proposed new address so the recipient can verify the
//!   change really came from them. We store the proposed new email in the
//!   magic-link row's `email` column (the row's `user_id` still ties the
//!   token to the caller).
//! - `POST  /users/me/email-change/confirm`  — atomically consume the token,
//!   replace `users.email`, and clear `email_verified_at` so the user must
//!   re-verify with the new address on next sign-in.
//!
//! Security: every endpoint here lives under `AuthMiddleware::required`. The
//! confirm step additionally cross-checks `record.user_id == claims.user_id`
//! so a leaked token never lets one user steal another's email.

use actix_web::{HttpRequest, get, patch, post, web};
use chrono::{DateTime, Duration, Utc};
use my_family_domain::{Locale, MagicLinkPurpose, MagicLinkRepoError, UserRepoError};
use my_family_email::{Locale as EmailLocale, render_email_change};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::{generate_opaque_token, hash_token};
use crate::validation::{
    email_invalid, email_same_as_current, locale_invalid, looks_like_email, string_too_long,
    value_required,
};
use crate::{ApiError, ApiResponse, AppState, response_body};

// ---------------------------------------------------------------------------
// Constants.
// ---------------------------------------------------------------------------

/// Maximum length we accept for `display_name`. Matches the FE's input bound.
const DISPLAY_NAME_MAX: u32 = 100;

// ---------------------------------------------------------------------------
// Request / response DTOs.
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub locale: String,
    pub timezone: String,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserReq {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct EmailChangeReq {
    pub new_email: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EmailChangeRes {
    pub status: &'static str,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct EmailChangeConfirmReq {
    pub token: String,
}

response_body!(pub UserProfileResponseBody, UserProfile);
response_body!(pub EmailChangeResponseBody, EmailChangeRes);

// ---------------------------------------------------------------------------
// Helpers.
// ---------------------------------------------------------------------------

fn seconds_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

fn parse_locale(raw: &str) -> Result<Locale, ApiError> {
    match raw {
        "en" => Ok(Locale::En),
        "de" => Ok(Locale::De),
        _ => Err(locale_invalid("/locale")),
    }
}

fn profile_from(user: &my_family_domain::User) -> UserProfile {
    UserProfile {
        id: user.id.into_uuid(),
        email: user.email.clone(),
        display_name: user.display_name.clone(),
        locale: user.locale.as_str().to_string(),
        timezone: user.timezone.clone(),
        email_verified_at: user.email_verified_at,
        created_at: user.created_at,
    }
}

// ---------------------------------------------------------------------------
// GET /users/me
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/users/me",
    operation_id = "user_me",
    responses(
        (status = 200, description = "Caller's profile", body = UserProfileResponseBody),
        (status = 401, description = "No session"),
    ),
    security(("cookie_access" = [])),
    tag = "users",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[get("/users/me")]
pub async fn me(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<ApiResponse<UserProfile>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let user = state
        .users
        .find_by_id(claims.user_id)
        .await
        .map_err(internal)?
        .ok_or(ApiError::Unauthenticated)?;
    Ok(ApiResponse::ok(profile_from(&user)))
}

// ---------------------------------------------------------------------------
// PATCH /users/me
// ---------------------------------------------------------------------------

#[utoipa::path(
    patch,
    path = "/api/v1/users/me",
    operation_id = "user_update",
    request_body = UpdateUserReq,
    responses(
        (status = 200, description = "Profile updated", body = UserProfileResponseBody),
        (status = 401, description = "No session"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "users",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[patch("/users/me")]
pub async fn update(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<UpdateUserReq>,
) -> Result<ApiResponse<UserProfile>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let payload = body.into_inner();

    // Validate before any DB writes — we want a single 422 envelope on bad
    // input, not a partially-applied patch.
    let trimmed_name = match payload.display_name.as_deref() {
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return Err(value_required("/display_name"));
            }
            if u32::try_from(trimmed.chars().count()).unwrap_or(u32::MAX) > DISPLAY_NAME_MAX {
                return Err(string_too_long("/display_name", DISPLAY_NAME_MAX));
            }
            Some(trimmed.to_string())
        }
        None => None,
    };
    let parsed_locale = match payload.locale.as_deref() {
        Some(raw) => Some(parse_locale(raw)?),
        None => None,
    };

    if trimmed_name.is_none() && parsed_locale.is_none() {
        return Err(value_required("/"));
    }

    if let Some(name) = trimmed_name.as_deref() {
        state.users.update_display_name(claims.user_id, name).await.map_err(internal)?;
    }
    if let Some(loc) = parsed_locale {
        state.users.update_locale(claims.user_id, loc).await.map_err(internal)?;
    }

    let user = state
        .users
        .find_by_id(claims.user_id)
        .await
        .map_err(internal)?
        .ok_or(ApiError::Unauthenticated)?;
    Ok(ApiResponse::ok(profile_from(&user)))
}

// ---------------------------------------------------------------------------
// POST /users/me/email-change
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/users/me/email-change",
    request_body = EmailChangeReq,
    responses(
        (status = 200, description = "Confirmation link sent to current email", body = EmailChangeResponseBody),
        (status = 401, description = "No session"),
        (status = 409, description = "New email already in use by another account"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "users",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/users/me/email-change")]
pub async fn email_change_request(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<EmailChangeReq>,
) -> Result<ApiResponse<EmailChangeRes>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
    let new_email = body.new_email.trim().to_lowercase();
    if !looks_like_email(&new_email) {
        return Err(email_invalid("/new_email"));
    }

    let user = state
        .users
        .find_by_id(claims.user_id)
        .await
        .map_err(internal)?
        .ok_or(ApiError::Unauthenticated)?;

    if new_email.eq_ignore_ascii_case(&user.email) {
        return Err(email_same_as_current("/new_email"));
    }

    if state.users.find_by_email(&new_email).await.map_err(internal)?.is_some() {
        return Err(ApiError::EmailTaken { email: new_email });
    }

    // Issue a single-use token. We store the **new** email in the magic-link
    // row's `email` column; the row's `user_id` still ties the token to the
    // caller, so the confirm step can update `users.email` to whatever the
    // row holds without a separate column.
    let (token, hash) = generate_opaque_token();
    state
        .magic_links
        .create(
            Some(user.id),
            &new_email,
            &hash,
            MagicLinkPurpose::EmailChange,
            Utc::now() + Duration::seconds(seconds_i64(state.cfg.magic_link_ttl_seconds)),
        )
        .await
        .map_err(internal)?;

    // The link is sent to the user's **current** address so a stolen account
    // (e.g. a coffee-shop session) can't be hijacked into the attacker's
    // inbox. The body shows the proposed new address so the recipient can
    // verify they really initiated the change.
    let link = format!("{}/account/email-change/consume?token={}", state.cfg.web_public_url, token);
    let locale = EmailLocale::from_str_or_en(user.locale.as_str());
    let (subject, text_body) = render_email_change(locale, &link, &new_email).map_err(internal)?;
    // Outbox-enqueue (durable, async). The worker drains via SMTP.
    state
        .outbox
        .enqueue(&my_family_domain::EmailOutboxInsert {
            kind: my_family_domain::EmailOutboxKind::EMAIL_CHANGE.to_string(),
            to_addr: user.email.clone(),
            subject,
            text_body,
            html_body: None,
        })
        .await
        .map_err(internal)?;

    // Keep the standard `ApiResponse::ok` envelope so the wire shape is
    // uniform across the API. The HTTP status defaults to 200 via the
    // `Responder` impl; the body's `status: "pending"` field marks the
    // accepted-but-awaiting-confirmation state to FE clients.
    Ok(ApiResponse::ok(EmailChangeRes { status: "pending" }))
}

// ---------------------------------------------------------------------------
// POST /users/me/email-change/confirm
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/users/me/email-change/confirm",
    request_body = EmailChangeConfirmReq,
    responses(
        (status = 200, description = "Email changed; verification cleared", body = UserProfileResponseBody),
        (status = 401, description = "No session or token invalid"),
        (status = 409, description = "New email already in use"),
    ),
    security(("cookie_access" = [])),
    tag = "users",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/users/me/email-change/confirm")]
pub async fn email_change_confirm(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<EmailChangeConfirmReq>,
) -> Result<ApiResponse<UserProfile>, ApiError> {
    let claims = crate::auth::user_claims(&req)?;
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

    // Cross-check the token actually belongs to the authenticated caller.
    // A leaked token from another user must not let the attacker hijack the
    // victim's email via their own session.
    if record.purpose != MagicLinkPurpose::EmailChange || record.user_id != Some(claims.user_id) {
        return Err(ApiError::MagicLinkInvalid);
    }

    state.users.update_email(claims.user_id, &record.email).await.map_err(|e| match e {
        UserRepoError::DuplicateEmail => ApiError::EmailTaken { email: record.email.clone() },
        other => ApiError::Internal(anyhow::anyhow!(other.to_string())),
    })?;
    state.users.mark_email_unverified(claims.user_id).await.map_err(internal)?;

    let user = state
        .users
        .find_by_id(claims.user_id)
        .await
        .map_err(internal)?
        .ok_or(ApiError::Unauthenticated)?;
    Ok(ApiResponse::ok(profile_from(&user)))
}

// ---------------------------------------------------------------------------
// Tests — pure-logic helpers; HTTP integration tests live in `user_flows.rs`.
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
    use super::*;

    #[test]
    fn parse_locale_accepts_en_and_de() {
        assert_eq!(parse_locale("en").unwrap(), Locale::En);
        assert_eq!(parse_locale("de").unwrap(), Locale::De);
    }

    #[test]
    fn parse_locale_rejects_unknown() {
        match parse_locale("fr") {
            Err(ApiError::Validation(v)) => {
                assert_eq!(v[0].code, "validation.locale_invalid");
            }
            _ => panic!("expected Validation"),
        }
    }

    #[test]
    fn update_user_req_accepts_partial_body() {
        let r: UpdateUserReq =
            serde_json::from_value(serde_json::json!({"display_name": "Ada"})).unwrap();
        assert_eq!(r.display_name.as_deref(), Some("Ada"));
        assert!(r.locale.is_none());
    }

    #[test]
    fn update_user_req_accepts_empty_object() {
        let r: UpdateUserReq = serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(r.display_name.is_none());
        assert!(r.locale.is_none());
    }

    #[test]
    fn email_change_req_deserialises_from_new_email_field() {
        let r: EmailChangeReq =
            serde_json::from_value(serde_json::json!({"new_email": "x@example.com"})).unwrap();
        assert_eq!(r.new_email, "x@example.com");
    }

    #[test]
    fn email_change_res_serialises_with_status_field() {
        let v = serde_json::to_value(EmailChangeRes { status: "pending" }).unwrap();
        assert_eq!(v["status"], "pending");
    }

    #[test]
    fn seconds_i64_clamps_overflow_to_max() {
        assert_eq!(seconds_i64(0), 0);
        assert_eq!(seconds_i64(900), 900);
        assert_eq!(seconds_i64(u64::MAX), i64::MAX);
    }
}
