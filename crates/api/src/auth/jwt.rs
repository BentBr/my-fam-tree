//! Ed25519 (`EdDSA`) JWT issue + verify built on top of `jsonwebtoken`.
//!
//! Verification looks up the verifying key by the `kid` header so retired
//! kids can still validate tokens issued before rotation.

use std::sync::Arc;

use anyhow::{Context, anyhow};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, decode_header, encode,
};
use my_fam_tree_domain::clock::Clock;
use uuid::Uuid;

use super::claims::{FamilyClaim, JwtClaims};
use super::keys::JwtKeyset;

/// Clock-skew tolerance when checking `exp`.
const LEEWAY_SECONDS: i64 = 5;

pub struct JwtIssuer {
    keys: JwtKeyset,
    issuer: String,
    audience: String,
    access_ttl_seconds: i64,
    clock: Arc<dyn Clock>,
}

impl std::fmt::Debug for JwtIssuer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtIssuer")
            .field("issuer", &self.issuer)
            .field("audience", &self.audience)
            .field("access_ttl_seconds", &self.access_ttl_seconds)
            .field("clock", &self.clock)
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
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { keys, issuer, audience, access_ttl_seconds, clock }
    }

    /// Issue an access token signed with the active signing key.
    ///
    /// # Errors
    /// Returns an error if the active signing key cannot be parsed as a
    /// PKCS#8 Ed25519 PEM, or if JWT encoding fails.
    pub fn issue(
        &self,
        sub: Uuid,
        email: &str,
        locale: &str,
        families: Vec<FamilyClaim>,
    ) -> anyhow::Result<String> {
        let now = self.clock.now().unix_timestamp();
        let claims = JwtClaims {
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            sub,
            email: email.to_owned(),
            locale: locale.to_owned(),
            families,
            iat: now,
            exp: now.saturating_add(self.access_ttl_seconds),
            jti: Uuid::new_v4().to_string(),
        };
        let mut header = Header::new(Algorithm::EdDSA);
        header.kid = Some(self.keys.active_kid.clone());
        // jsonwebtoken wants PEM here, not the raw 32/64-byte Ed25519 key bytes.
        let encoding = EncodingKey::from_ed_pem(self.keys.signing_pem.as_bytes())
            .context("EncodingKey::from_ed_pem (Ed25519 PKCS#8 PEM expected)")?;
        encode(&header, &claims, &encoding).context("encode JWT")
    }

    /// Verify a token and return its claims.
    ///
    /// # Errors
    /// Returns an error if the token has no `kid` header, references an
    /// unknown key, fails signature / issuer / audience / expiry checks, or
    /// the corresponding verifying key cannot be parsed.
    pub fn verify(&self, token: &str) -> anyhow::Result<JwtClaims> {
        let header = decode_header(token).context("invalid JWT header")?;
        let kid = header.kid.ok_or_else(|| anyhow!("JWT missing kid"))?;
        let verify_pem =
            self.keys.verifying_pem.get(&kid).ok_or_else(|| anyhow!("unknown kid: {kid}"))?;
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        // Expiry is checked against the injected clock below, not by
        // jsonwebtoken (which reads system time). `exp` stays a required claim.
        validation.validate_exp = false;
        validation.validate_nbf = false;
        let decoding = DecodingKey::from_ed_pem(verify_pem.as_bytes())
            .context("DecodingKey::from_ed_pem (Ed25519 SPKI PEM expected)")?;
        let data = decode::<JwtClaims>(token, &decoding, &validation).context("verify JWT")?;
        let now = self.clock.now().unix_timestamp();
        if data.claims.exp.saturating_add(LEEWAY_SECONDS) < now {
            return Err(anyhow!("verify JWT: token expired"));
        }
        Ok(data.claims)
    }
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::pkcs8::spki::der::pem::LineEnding;
    use ed25519_dalek::pkcs8::{EncodePrivateKey, EncodePublicKey};
    use my_fam_tree_domain::clock::FixedClock;
    use rand::rand_core::UnwrapErr;
    use rand::rngs::SysRng;
    use time::{Duration, OffsetDateTime};

    use super::*;

    fn fixture_issuer(kid: &str) -> JwtIssuer {
        fixture_issuer_with_clock(kid, Arc::new(my_fam_tree_domain::clock::SystemClock))
    }

    fn fixture_issuer_with_clock(kid: &str, clock: Arc<dyn Clock>) -> JwtIssuer {
        let sk = ed25519_dalek::SigningKey::generate(&mut UnwrapErr(SysRng));
        let priv_pem = sk.to_pkcs8_pem(LineEnding::LF).unwrap().to_string();
        let pub_pem = sk.verifying_key().to_public_key_pem(LineEnding::LF).unwrap();
        let public_json =
            serde_json::json!([{"kid": kid, "public_pem": pub_pem.trim_end()}]).to_string();
        let keys = JwtKeyset::load(&priv_pem, kid, &public_json).unwrap();
        JwtIssuer::new(keys, "iss".into(), "aud".into(), 900, clock)
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
                    role: my_fam_tree_domain::Role::Owner,
                }],
            )
            .unwrap();
        let claims = issuer.verify(&token).unwrap();
        assert_eq!(claims.email, "a@b.c");
        assert_eq!(claims.families.len(), 1);
        assert_eq!(claims.families[0].role, my_fam_tree_domain::Role::Owner);
    }

    #[test]
    fn rejects_wrong_audience() {
        let mut issuer = fixture_issuer("kid-1");
        let token = issuer.issue(Uuid::new_v4(), "a", "en", vec![]).unwrap();
        issuer.audience = "other-aud".into();
        assert!(issuer.verify(&token).is_err());
    }

    const T: OffsetDateTime = time::macros::datetime!(2040-01-01 00:00 UTC);

    #[test]
    fn jwt_follows_injected_clock() {
        let clock = FixedClock::new(T);
        let issuer = fixture_issuer_with_clock("kid-1", Arc::new(clock.clone()));
        let token = issuer.issue(Uuid::new_v4(), "a", "en", vec![]).unwrap();

        clock.set(T + Duration::seconds(900 - 1));
        let claims = issuer.verify(&token).expect("valid just before exp by the injected clock");
        assert_eq!(claims.iat, T.unix_timestamp(), "iat comes from the injected clock");
        assert_eq!(claims.exp, T.unix_timestamp() + 900, "exp = iat + access ttl");

        clock.set(T + Duration::seconds(900 + 5 + 1));
        assert!(issuer.verify(&token).is_err(), "expired past ttl + leeway by the injected clock");
    }

    #[test]
    fn verification_ignores_system_time() {
        let clock = FixedClock::new(my_fam_tree_domain::clock::SystemClock.now());
        let issuer = fixture_issuer_with_clock("kid-1", Arc::new(clock.clone()));
        let token = issuer.issue(Uuid::new_v4(), "a", "en", vec![]).unwrap();

        clock.set(T);
        assert!(
            issuer.verify(&token).is_err(),
            "a token issued at real time is expired when the clock reads 2040"
        );
    }
}
