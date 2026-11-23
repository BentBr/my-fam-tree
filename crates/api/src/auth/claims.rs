//! JWT claim shape.
//!
//! Includes the `families` array as per spec Section 6: every membership
//! the user has is embedded in the access token so the FE can render the
//! family picker without an extra round-trip and the middleware can resolve
//! `X-Family-Id` against the token without hitting the DB.

use my_fam_tree_domain::Role;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FamilyClaim {
    pub id: Uuid,
    pub name: String,
    #[serde(serialize_with = "ser_role", deserialize_with = "de_role")]
    pub role: Role,
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde with-style hooks require &T")]
fn ser_role<S: serde::Serializer>(r: &Role, s: S) -> Result<S::Ok, S::Error> {
    let v = match r {
        Role::User => "user",
        Role::Admin => "admin",
        Role::Owner => "owner",
    };
    s.serialize_str(v)
}

fn de_role<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Role, D::Error> {
    let v = String::deserialize(d)?;
    match v.as_str() {
        "user" => Ok(Role::User),
        "admin" => Ok(Role::Admin),
        "owner" => Ok(Role::Owner),
        other => Err(serde::de::Error::custom(format!("unknown role: {other}"))),
    }
}

fn default_edge_style() -> String {
    my_fam_tree_domain::EdgeStyle::Curved.as_str().to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JwtClaims {
    pub iss: String,
    pub aud: String,
    pub sub: Uuid,
    pub email: String,
    pub locale: String,
    /// Tree connector style (`"curved"` / `"angular"`). Defaulted so access
    /// tokens minted before this claim existed still verify.
    #[serde(default = "default_edge_style")]
    pub edge_style: String,
    pub families: Vec<FamilyClaim>,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
}

impl JwtClaims {
    #[must_use]
    pub fn family(&self, family_id: Uuid) -> Option<&FamilyClaim> {
        self.families.iter().find(|f| f.id == family_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_minted_before_edge_style_existed_defaults_to_curved() {
        let legacy = serde_json::json!({
            "iss": "i", "aud": "a", "sub": Uuid::nil(), "email": "a@b.c", "locale": "de",
            "families": [], "iat": 0, "exp": 0, "jti": "j",
        });
        let claims: JwtClaims = serde_json::from_value(legacy).expect("legacy claims parse");
        assert_eq!(claims.edge_style, "curved");
    }

    #[test]
    fn edge_style_claim_round_trips() {
        let json = serde_json::json!({
            "iss": "i", "aud": "a", "sub": Uuid::nil(), "email": "a@b.c", "locale": "en",
            "edge_style": "angular", "families": [], "iat": 0, "exp": 0, "jti": "j",
        });
        let claims: JwtClaims = serde_json::from_value(json).expect("claims parse");
        assert_eq!(claims.edge_style, "angular");
    }
}
