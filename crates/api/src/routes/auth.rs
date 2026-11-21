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

mod request_link;
mod session;
mod token;

use std::time::Duration as StdDuration;

use actix_web::{HttpRequest, web};
pub use request_link::{__path_magic_link, magic_link};
use serde::{Deserialize, Serialize};
pub use session::{__path_logout, __path_me, logout, me};
pub use token::{__path_consume, __path_refresh, consume, refresh};
use utoipa::ToSchema;

use crate::auth::FamilyClaim;
use crate::{ApiError, AppState, response_body};

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
    /// Locale the FE detected on the sign-in page (`"en"` / `"de"`). Optional
    /// for backwards-compatible callers; used to seed a brand-new account's
    /// preference AND to locale-prefix the magic-link URL. For a RETURNING
    /// user their stored account locale wins (it's their explicit setting),
    /// so this only takes effect when the account doesn't exist yet.
    #[serde(default)]
    pub locale: Option<String>,
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
