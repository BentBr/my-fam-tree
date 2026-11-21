//! `POST /auth/magic-link` — issue + email a single-use sign-in link.

#![expect(
    clippy::future_not_send,
    reason = "actix handlers hold !Send request types (HttpRequest, Multipart); actix runs each worker on a single thread"
)]
#![expect(
    unreachable_pub,
    reason = "actix route macros turn each handler into a pub service struct"
)]

use std::time::Duration as StdDuration;

use actix_web::{HttpRequest, post, web};
use my_fam_tree_domain::Locale;
use my_fam_tree_email::{Locale as EmailLocale, render_magic_link, render_welcome};

use super::{MagicLinkReq, MagicLinkRes, MagicLinkResponseBody};
use crate::services::auth_service::mint_magic_link_url;
use crate::validation::{email_invalid, looks_like_email};
use crate::{ApiError, ApiResponse, AppState};

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
    // `is_new_user` records whether THIS request created the account so we
    // can send the welcome/onboarding variant instead of the bare sign-in
    // email (both carry the same single-use link).
    let (user, is_new_user) = match state
        .users
        .find_by_email(&email)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?
    {
        Some(u) => (u, false),
        None => (
            // Seed the new account with the locale the FE detected on the
            // sign-in page (defaults to En for legacy callers that omit it).
            // A returning user's stored preference is left untouched below.
            state
                .users
                .create(&email, Locale::from_str_or_en(body.locale.as_deref().unwrap_or("en")))
                .await
                .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?,
            true,
        ),
    };

    // The email locale + URL prefix both follow the user's effective locale:
    // their stored account setting for a returning user, the just-seeded
    // request locale for a fresh account.
    let locale = EmailLocale::from_str_or_en(user.locale.as_str());

    // Issue and persist a single-use token; the helper returns the full,
    // locale-prefixed consume URL (only the hash hits the DB).
    let link = mint_magic_link_url(
        &state.magic_links,
        user.id,
        &user.email,
        &state.cfg.web.public_url,
        locale,
        state.cfg.magic_link.ttl_seconds,
    )
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;

    // Render the email + enqueue it into the durable outbox. Postgres is
    // the source of truth, so a Redis flush can't lose mail and SMTP
    // slowness no longer blocks this request thread — the worker drains
    // the outbox out-of-band via SMTP with retry/backoff.
    let app_url = state.cfg.web.public_url.as_str();
    // Brand-new accounts get the welcome/onboarding copy; returning users
    // get the plain sign-in email, greeted by name when we have one. Both
    // embed the same single-use link.
    let msg = if is_new_user {
        render_welcome(locale, app_url, &link)
    } else {
        let name = Some(user.display_name.trim()).filter(|n| !n.is_empty());
        render_magic_link(locale, app_url, &link, name)
    }
    .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;
    state
        .outbox
        .enqueue(&my_fam_tree_domain::EmailOutboxInsert {
            kind: my_fam_tree_domain::EmailOutboxKind::MAGIC_LINK.to_string(),
            to_addr: user.email.clone(),
            subject: msg.subject,
            text_body: msg.text,
            html_body: Some(msg.html),
        })
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))?;

    Ok(ApiResponse::ok(MagicLinkRes { status: "sent" }))
}
