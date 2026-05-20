//! Request-scoped mirror of a verified JWT.
//!
//! The auth middleware verifies the access cookie, decodes the `JwtClaims`
//! payload, optionally resolves `X-Family-Id` against `claims.families`, and
//! inserts a `UserClaims` value into the request extensions. Handlers read it
//! back via `crate::auth::user_claims(&req)` — branded ID types are used so
//! handlers can't accidentally mix `UserId` with `FamilyId`.

use my_family_domain::{FamilyId, Role, UserId};

#[derive(Debug, Clone)]
pub struct UserClaims {
    pub user_id: UserId,
    pub email: String,
    pub locale: String,
    /// Present iff the request carried a valid `X-Family-Id` header that
    /// matched one of the user's memberships in the JWT.
    pub active_family: Option<ActiveFamily>,
    /// Every family membership the JWT claims, in token order.
    pub all_families: Vec<FamilyMembershipMirror>,
}

#[derive(Debug, Clone)]
pub struct ActiveFamily {
    pub id: FamilyId,
    pub name: String,
    pub role: Role,
}

#[derive(Debug, Clone)]
pub struct FamilyMembershipMirror {
    pub id: FamilyId,
    pub name: String,
    pub role: Role,
}
