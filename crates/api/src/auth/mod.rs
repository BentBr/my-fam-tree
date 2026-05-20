//! Authentication primitives: Ed25519 JWT keyset, claim shape, issue/verify,
//! and opaque refresh-token helpers. Handlers and middleware in 1b consume
//! these via `crate::auth::*` re-exports.

pub mod claims;
pub mod jwt;
pub mod keys;
pub mod tokens;

pub use claims::{FamilyClaim, JwtClaims};
pub use jwt::JwtIssuer;
pub use keys::JwtKeyset;
pub use tokens::{generate_opaque_token, hash_token};
