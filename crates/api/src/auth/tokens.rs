//! Opaque refresh-token helpers.
//!
//! Tokens are 256-bit url-safe base64 strings; only the SHA-256 hash is
//! persisted so a DB leak can't be used to forge a session.

use rand::Rng;
use rand::rand_core::UnwrapErr;
use rand::rngs::SysRng;
use sha2::{Digest, Sha256};

/// Generates a 256-bit opaque token; returns (`urlsafe-b64` string, sha256 hash bytes).
#[must_use]
pub fn generate_opaque_token() -> (String, Vec<u8>) {
    let mut bytes = [0u8; 32];
    // OS CSPRNG; `UnwrapErr` panics only if the OS RNG itself fails.
    UnwrapErr(SysRng).fill_bytes(&mut bytes);
    let token = base64_urlsafe(&bytes);
    let hash = Sha256::digest(token.as_bytes()).to_vec();
    (token, hash)
}

#[must_use]
pub fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

fn base64_urlsafe(bytes: &[u8]) -> String {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn token_and_hash_are_deterministic() {
        let (tok, hash) = generate_opaque_token();
        assert_eq!(hash, hash_token(&tok));
        assert_eq!(hash.len(), 32);
    }

    /// 32 random bytes → 43 chars of URL-safe base64 without padding.
    /// Pins the wire format across `base64` / `rand` upgrades.
    #[test]
    fn token_is_43_char_urlsafe_base64_without_padding() {
        let (tok, _) = generate_opaque_token();
        assert_eq!(tok.len(), 43);
        assert!(tok.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'), "{tok}");
        assert_eq!(base64_urlsafe(&[0xfb, 0xff]), "-_8");
    }

    #[test]
    fn tokens_are_unique() {
        let (a, _) = generate_opaque_token();
        let (b, _) = generate_opaque_token();
        assert_ne!(a, b);
    }
}
