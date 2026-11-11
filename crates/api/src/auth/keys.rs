//! Ed25519 key material loaded from env.
//!
//! `JWT_PRIVATE_KEY` holds a single PKCS#8 PEM (the active signing key).
//! `JWT_PUBLIC_KEYS` is a JSON array `[{kid, public_pem}, ...]` that
//! includes the active kid plus any retired kids still accepted at verify
//! time. `\n` escape sequences in the env values are unescaped so the
//! variables can live on a single line in `.env`.

use std::collections::HashMap;

use anyhow::{Context, anyhow};
use ed25519_dalek::pkcs8::{DecodePrivateKey, DecodePublicKey};
use ed25519_dalek::{SigningKey, VerifyingKey};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PublicKeyEntry {
    kid: String,
    public_pem: String,
}

/// Per-kid Ed25519 key storage.
///
/// Holds the PEM bytes so we can hand them straight to
/// `jsonwebtoken::EncodingKey::from_ed_pem` / `DecodingKey::from_ed_pem` —
/// those expect PEM, NOT the raw 32/64-byte Ed25519 key material.
#[derive(Clone)]
pub struct JwtKeyset {
    pub active_kid: String,
    /// PKCS#8 PEM (private key).
    pub signing_pem: String,
    /// Kept for tests + key fingerprint use cases.
    pub signing: SigningKey,
    /// SPKI PEM per kid.
    pub verifying_pem: HashMap<String, String>,
    pub verifying: HashMap<String, VerifyingKey>,
}

impl std::fmt::Debug for JwtKeyset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Intentionally elide the signing key + PEM material from Debug output.
        f.debug_struct("JwtKeyset")
            .field("active_kid", &self.active_kid)
            .field("verifying_kids", &self.verifying.keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl JwtKeyset {
    /// Parses the active signing key and the JSON array of verifying keys.
    ///
    /// `private_pem` and any `public_pem` may contain literal `\n` sequences
    /// (as produced by `gen-jwt-keys`); those get unescaped before parsing.
    ///
    /// # Errors
    /// Returns an error if `JWT_PRIVATE_KEY` is not a valid Ed25519 PKCS#8
    /// PEM, if `JWT_PUBLIC_KEYS` cannot be parsed as a JSON array of
    /// `{kid, public_pem}` entries, if any `public_pem` is invalid SPKI, or
    /// if `active_kid` is missing from the verifying-key set.
    pub fn load(
        private_pem: &str,
        active_kid: &str,
        public_keys_json: &str,
    ) -> anyhow::Result<Self> {
        let signing_pem = private_pem.replace("\\n", "\n");
        let signing = SigningKey::from_pkcs8_pem(&signing_pem)
            .context("failed to parse JWT_PRIVATE_KEY (expected Ed25519 PKCS#8 PEM)")?;

        let entries: Vec<PublicKeyEntry> = serde_json::from_str(public_keys_json)
            .context("JWT_PUBLIC_KEYS must be a JSON array of {kid, public_pem}")?;

        let mut verifying = HashMap::new();
        let mut verifying_pem = HashMap::new();
        for e in entries {
            let pem = e.public_pem.replace("\\n", "\n");
            let key = VerifyingKey::from_public_key_pem(&pem)
                .with_context(|| format!("invalid public PEM for kid={}", e.kid))?;
            verifying.insert(e.kid.clone(), key);
            verifying_pem.insert(e.kid, pem);
        }
        if !verifying.contains_key(active_kid) {
            return Err(anyhow!("JWT_PRIVATE_KEY_ID {active_kid} not present in JWT_PUBLIC_KEYS"));
        }
        Ok(Self {
            active_kid: active_kid.to_string(),
            signing_pem,
            signing,
            verifying_pem,
            verifying,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::auth::jwt::JwtIssuer;

    // Fixed Ed25519 key pair from RFC 8410 §10.1 / §10.3, written exactly the
    // way `gen-jwt-keys` emits them for `.env` (single line, literal `\n`).
    // Every other test generates a fresh key with the *current* ed25519-dalek;
    // these pin that keys produced by any earlier version (i.e. the ones
    // already deployed) still parse and sign after a dependency bump.
    const PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----\\nMC4CAQAwBQYDK2VwBCIEINTuctv5E1hK1bbY8fdp+K06/nwoy/HU++CXqI9EdVhC\\n-----END PRIVATE KEY-----\\n";
    const PUBLIC_PEM: &str = "-----BEGIN PUBLIC KEY-----\\nMCowBQYDK2VwAyEAGb9ECWmEzf6FQbrBZ9w7lshQhqowtrbLDFw4rXAxZuE=\\n-----END PUBLIC KEY-----\\n";

    fn public_json(kid: &str) -> String {
        format!(r#"[{{"kid":"{kid}","public_pem":"{PUBLIC_PEM}"}}]"#)
    }

    #[test]
    fn loads_fixed_rfc8410_key_pair_from_env_format() {
        let keys = JwtKeyset::load(PRIVATE_PEM, "k1", &public_json("k1")).unwrap();
        assert_eq!(keys.active_kid, "k1");
        // The private key must derive exactly the published public key.
        assert_eq!(keys.signing.verifying_key(), keys.verifying["k1"]);
        assert!(!keys.signing_pem.contains("\\n"), "escaped newlines are unescaped");
    }

    #[test]
    fn fixed_key_pair_signs_and_verifies_jwt() {
        let keys = JwtKeyset::load(PRIVATE_PEM, "k1", &public_json("k1")).unwrap();
        let issuer = JwtIssuer::new(keys, "iss".into(), "aud".into(), 900);
        let token = issuer.issue(uuid::Uuid::new_v4(), "a@b.c", "en", vec![]).unwrap();
        assert_eq!(issuer.verify(&token).unwrap().email, "a@b.c");
    }

    #[test]
    fn rejects_active_kid_missing_from_public_keys() {
        let err = JwtKeyset::load(PRIVATE_PEM, "k2", &public_json("k1")).unwrap_err();
        assert!(err.to_string().contains("k2"), "{err}");
    }

    #[test]
    fn rejects_malformed_private_pem() {
        assert!(JwtKeyset::load("not a pem", "k1", &public_json("k1")).is_err());
    }
}
