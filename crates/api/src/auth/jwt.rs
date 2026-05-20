//! Ed25519 (`EdDSA`) JWT issue + verify built on top of `jsonwebtoken`.
//!
//! Verification looks up the verifying key by the `kid` header so retired
//! kids can still validate tokens issued before rotation.

use anyhow::{Context, anyhow};
use chrono::Utc;
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, decode_header, encode,
};
use uuid::Uuid;

use super::claims::{FamilyClaim, JwtClaims};
use super::keys::JwtKeyset;

pub struct JwtIssuer {
    keys: JwtKeyset,
    issuer: String,
    audience: String,
    access_ttl_seconds: i64,
}

impl std::fmt::Debug for JwtIssuer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtIssuer")
            .field("issuer", &self.issuer)
            .field("audience", &self.audience)
            .field("access_ttl_seconds", &self.access_ttl_seconds)
            .field("keys", &self.keys)
            .finish()
    }
}

impl JwtIssuer {
    #[must_use]
    pub const fn new(
        keys: JwtKeyset,
        issuer: String,
        audience: String,
        access_ttl_seconds: i64,
    ) -> Self {
        Self { keys, issuer, audience, access_ttl_seconds }
    }

    pub fn issue(
        &self,
        sub: Uuid,
        email: &str,
        locale: &str,
        families: Vec<FamilyClaim>,
    ) -> anyhow::Result<String> {
        let now = Utc::now().timestamp();
        let claims = JwtClaims {
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            sub,
            email: email.to_string(),
            locale: locale.to_string(),
            families,
            iat: now,
            exp: now + self.access_ttl_seconds,
            jti: Uuid::new_v4().to_string(),
        };
        let mut header = Header::new(Algorithm::EdDSA);
        header.kid = Some(self.keys.active_kid.clone());
        // jsonwebtoken wants PEM here, not the raw 32/64-byte Ed25519 key bytes.
        let encoding = EncodingKey::from_ed_pem(self.keys.signing_pem.as_bytes())
            .context("EncodingKey::from_ed_pem (Ed25519 PKCS#8 PEM expected)")?;
        encode(&header, &claims, &encoding).context("encode JWT")
    }

    pub fn verify(&self, token: &str) -> anyhow::Result<JwtClaims> {
        let header = decode_header(token).context("invalid JWT header")?;
        let kid = header.kid.ok_or_else(|| anyhow!("JWT missing kid"))?;
        let verify_pem =
            self.keys.verifying_pem.get(&kid).ok_or_else(|| anyhow!("unknown kid: {kid}"))?;
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        validation.leeway = 5;
        let decoding = DecodingKey::from_ed_pem(verify_pem.as_bytes())
            .context("DecodingKey::from_ed_pem (Ed25519 SPKI PEM expected)")?;
        let data = decode::<JwtClaims>(token, &decoding, &validation).context("verify JWT")?;
        Ok(data.claims)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
    use ed25519_dalek::pkcs8::{EncodePrivateKey, EncodePublicKey};
    use rand::rngs::OsRng;

    use super::*;

    fn fixture_issuer(kid: &str) -> JwtIssuer {
        let sk = ed25519_dalek::SigningKey::generate(&mut OsRng);
        let priv_pem = sk.to_pkcs8_pem(LineEnding::LF).unwrap().to_string();
        let pub_pem = sk.verifying_key().to_public_key_pem(LineEnding::LF).unwrap();
        let public_json =
            serde_json::json!([{"kid": kid, "public_pem": pub_pem.trim_end()}]).to_string();
        let keys = JwtKeyset::load(&priv_pem, kid, &public_json).unwrap();
        JwtIssuer::new(keys, "iss".into(), "aud".into(), 900)
    }

    #[test]
    fn round_trip_with_families() {
        let issuer = fixture_issuer("kid-1");
        let token = issuer
            .issue(
                Uuid::new_v4(),
                "a@b.c",
                "de",
                vec![FamilyClaim {
                    id: Uuid::new_v4(),
                    name: "Müller".into(),
                    role: my_family_domain::Role::Owner,
                }],
            )
            .unwrap();
        let claims = issuer.verify(&token).unwrap();
        assert_eq!(claims.email, "a@b.c");
        assert_eq!(claims.families.len(), 1);
        assert_eq!(claims.families[0].role, my_family_domain::Role::Owner);
    }

    #[test]
    fn rejects_wrong_audience() {
        let mut issuer = fixture_issuer("kid-1");
        let token = issuer.issue(Uuid::new_v4(), "a", "en", vec![]).unwrap();
        issuer.audience = "other-aud".into();
        assert!(issuer.verify(&token).is_err());
    }
}
