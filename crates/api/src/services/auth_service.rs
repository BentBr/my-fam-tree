//! Auth domain orchestration.
//!
//! Pure service-layer helpers that combine repos and the JWT issuer so HTTP
//! handlers (`/auth/consume`, `/auth/refresh`, `/families`, `/invites/accept`)
//! mint a fresh access token with the same family-claim bundling logic.

use std::sync::Arc;

use anyhow::Context;
use chrono::{Duration, Utc};
use my_fam_tree_config::JwtConfig;
use my_fam_tree_domain::{
    FamilyMembershipRepo, MagicLinkPurpose, MagicLinkRepo, RefreshTokenRepo, User, UserId,
};

use crate::auth::{FamilyClaim, JwtIssuer, generate_opaque_token};

/// Issue a signed access JWT for `user`, embedding every family they belong to.
///
/// Returns the encoded token plus the same `FamilyClaim` vector so callers can
/// echo it in their JSON response without a second DB round-trip.
///
/// # Errors
/// Propagates DB errors from the memberships repo and signing errors from the
/// JWT issuer.
pub async fn issue_access_token_for(
    issuer: &JwtIssuer,
    memberships: &Arc<dyn FamilyMembershipRepo>,
    user: &User,
) -> anyhow::Result<(String, Vec<FamilyClaim>)> {
    let claims: Vec<FamilyClaim> = memberships
        .list_for_user(user.id)
        .await
        .context("list memberships")?
        .into_iter()
        .map(|m| FamilyClaim { id: m.family_id.into_uuid(), name: m.family_name, role: m.role })
        .collect();

    let token =
        issuer.issue(user.id.into_uuid(), &user.email, user.locale.as_str(), claims.clone())?;
    Ok((token, claims))
}

/// Mint + persist a fresh opaque refresh token for `user`, returning the
/// raw token string (the cookie value the BE will Set-Cookie to the
/// browser).
///
/// Used by every "first-time-session" path:
///   - `POST /auth/consume` after a magic-link sign-in
///   - `POST /invites/accept` after the recipient accepts an invite
///
/// `POST /auth/refresh` is a separate flow (it ROTATES an existing
/// row via `refresh_tokens.rotate`) and doesn't share this helper.
///
/// Persists only the sha256 hash of the token. The raw token never
/// goes to disk — it's encoded into the refresh cookie and only the
/// client holds it. Rolling + absolute deadlines come from JWT
/// config (`refresh_ttl_seconds` + `refresh_absolute_ttl_seconds`).
///
/// # Errors
/// Propagates DB errors from the refresh-token repo as an
/// `anyhow::Error`; callers wrap into their handler's `ApiError`.
pub async fn mint_refresh_token_for(
    refresh_tokens: &Arc<dyn RefreshTokenRepo>,
    jwt_cfg: &JwtConfig,
    user: &User,
) -> anyhow::Result<String> {
    let (token, hash) = generate_opaque_token();
    let now = Utc::now();
    let ttl = i64::try_from(jwt_cfg.refresh_ttl_seconds).unwrap_or(i64::MAX);
    let abs = i64::try_from(jwt_cfg.refresh_absolute_ttl_seconds).unwrap_or(i64::MAX);
    refresh_tokens
        .create(
            user.id,
            &hash,
            None,
            None,
            None,
            now + Duration::seconds(ttl),
            now + Duration::seconds(abs),
        )
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("persist refresh token")?;
    Ok(token)
}

/// Mint and persist a single-use magic-link `Login` token for `user_id`,
/// returning the full consume URL built from `web_public_url`.
///
/// Shared by `POST /auth/magic-link` (which then renders + sends an email) and
/// by `crates/api/src/seed.rs` (which only needs the URL for stdout). Persists
/// only the sha256 hash; the opaque token bytes are encoded into the returned
/// URL and never written to the DB.
///
/// # Errors
/// Propagates DB errors from the magic-link repo. The expiry overflow is
/// silently clamped to `i64::MAX` seconds (same behaviour as the HTTP handler).
pub async fn mint_magic_link_url(
    magic_links: &Arc<dyn MagicLinkRepo>,
    user_id: UserId,
    email: &str,
    web_public_url: &str,
    ttl_seconds: u64,
) -> anyhow::Result<String> {
    let (token, hash) = generate_opaque_token();
    let ttl = i64::try_from(ttl_seconds).unwrap_or(i64::MAX);
    magic_links
        .create(
            Some(user_id),
            email,
            &hash,
            MagicLinkPurpose::Login,
            Utc::now() + Duration::seconds(ttl),
        )
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))
        .context("persist magic-link token")?;
    Ok(format!("{web_public_url}/auth/consume?token={token}"))
}
