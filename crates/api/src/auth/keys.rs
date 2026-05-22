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
