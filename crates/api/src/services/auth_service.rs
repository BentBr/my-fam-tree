//! Auth domain orchestration.
//!
//! Pure service-layer helpers that combine repos and the JWT issuer so HTTP
//! handlers (`/auth/consume`, `/auth/refresh`, `/families`, `/invites/accept`)
//! mint a fresh access token with the same family-claim bundling logic.

use std::sync::Arc;

use anyhow::Context;
use my_family_domain::{FamilyMembershipRepo, User};

use crate::auth::{FamilyClaim, JwtIssuer};

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

    let token = issuer.issue(
        user.id.into_uuid(),
        &user.email,
        user.locale.as_str(),
        claims.clone(),
    )?;
    Ok((token, claims))
}
