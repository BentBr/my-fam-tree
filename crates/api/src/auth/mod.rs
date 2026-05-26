//! Authentication primitives.
//!
//! Ed25519 JWT keyset, claim shape, issue/verify, opaque refresh-token helpers,
//! plus the Actix middleware that verifies the access cookie and resolves
//! `X-Family-Id` against the JWT memberships.

pub mod claims;
pub mod jwt;
pub mod keys;
pub mod middleware;
pub mod tokens;
pub mod user_claims;

pub use claims::{FamilyClaim, JwtClaims};
pub use jwt::JwtIssuer;
pub use keys::JwtKeyset;
pub use middleware::{
    AuthMiddleware, FAMILY_HEADER, require_role, try_user_claims, user_claims,
    user_claims_with_family,
};
pub use tokens::{generate_opaque_token, hash_token};
pub use user_claims::{ActiveFamily, FamilyMembershipMirror, UserClaims};
