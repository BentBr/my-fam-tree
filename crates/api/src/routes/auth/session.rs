//! `POST /auth/logout` and `GET /auth/me`.

#![expect(
    clippy::future_not_send,
    reason = "actix handlers hold !Send request types (HttpRequest, Multipart); actix runs each worker on a single thread"
)]
#![expect(
    unreachable_pub,
    reason = "actix route macros turn each handler into a pub service struct"
)]

use actix_web::{HttpRequest, HttpResponse, post, web};

use super::{ConsumeRes, LogoutRes, LogoutResponseBody, MeResponseBody, build_consume_response};
use crate::auth::{FamilyClaim, hash_token};
use crate::cookies::{ACCESS_COOKIE, REFRESH_COOKIE, REFRESH_COOKIE_PATH, revoked};
use crate::{ApiError, ApiResponse, AppState};

// ---------------------------------------------------------------------------
// POST /auth/logout
// ---------------------------------------------------------------------------

/// POST `/auth/logout` — clear the session.
///
/// Public + idempotent: mounted OUTSIDE the required-auth scope (see
/// `routes::mod`) so the FE can call it on any "session is gone"
/// signal — including the case where the access cookie has already
/// expired and an auth-gated logout would 401.
///
/// Behaviour:
///   - if the refresh cookie is present, best-effort revoke the
///     matching row in the DB;
///   - always emit `Set-Cookie max-age=0` for both cookies so the
///     browser drops them;
///   - return 200 regardless.
///
/// Response body is a fixed `{ status: "logged out" }` for every
/// caller and the Set-Cookie clearing headers carry no session
/// information, so public access reveals nothing an unauthenticated
/// probe could not already observe.
//
// The doc above intentionally runs long — the security rationale for
// "why public" lives at the call site so future reviewers can audit
// without chasing references. Localised allow keeps the nursery lint
// in place for other items where shorter docs are still preferable.
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    responses(
        (status = 200, description = "Logged out (or no session to begin with)", body = LogoutResponseBody),
    ),
    tag = "auth",
)]
#[post("/auth/logout")]
pub async fn logout(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    if let Some(c) = req.cookie(REFRESH_COOKIE) {
        let hash = hash_token(c.value());
        // Best-effort: a refresh-row revoke failure must not block logout.
        if let Err(e) = state.refresh_tokens.revoke_by_hash(&hash).await {
            tracing::warn!(error = %e, "logout: refresh token revoke failed");
        }
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
