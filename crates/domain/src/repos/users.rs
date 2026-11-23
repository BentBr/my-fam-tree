use async_trait::async_trait;
use time::OffsetDateTime;

use crate::UserId;

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub display_name: String,
    pub locale: Locale,
    pub edge_style: EdgeStyle,
    pub timezone: String,
    pub email_verified_at: Option<OffsetDateTime>,
    /// Opaque object-storage key for this user's avatar, or `None` when
    /// they have no avatar yet. Resolved to a presigned URL at the HTTP edge.
    pub avatar_key: Option<String>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    De,
}

impl Locale {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
        }
    }

    /// Parse `"en"` / `"de"` (case-insensitive), falling back to [`Locale::En`]
    /// for anything else — used to seed a brand-new account from the locale the
    /// FE detected on the sign-in page.
    #[must_use]
    pub const fn from_str_or_en(s: &str) -> Self {
        if s.as_bytes().eq_ignore_ascii_case(b"de") { Self::De } else { Self::En }
    }
}

/// How the family tree draws parent → child connectors. An account
/// preference, stored and transported exactly like [`Locale`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeStyle {
    /// Smooth Bezier curves — the default.
    Curved,
    /// Right-angled elbows.
    Angular,
}

impl EdgeStyle {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Curved => "curved",
            Self::Angular => "angular",
        }
    }

    /// Parse `"curved"` / `"angular"`; `None` for anything else.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "curved" => Some(Self::Curved),
            "angular" => Some(Self::Angular),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UserRepoError {
    #[error("database: {0}")]
    Db(String),
    #[error("not found")]
    NotFound,
    #[error("email already in use")]
    DuplicateEmail,
}

#[async_trait]
pub trait UserRepo: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, UserRepoError>;
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, UserRepoError>;
    async fn create(&self, email: &str, locale: Locale) -> Result<User, UserRepoError>;
    async fn mark_verified(&self, id: UserId) -> Result<(), UserRepoError>;
    async fn update_locale(&self, id: UserId, locale: Locale) -> Result<(), UserRepoError>;
    async fn update_edge_style(
        &self,
        id: UserId,
        edge_style: EdgeStyle,
    ) -> Result<(), UserRepoError>;
    /// Update the user's display name. The caller is responsible for trimming
    /// + length validation; the repo simply writes the value through.
    async fn update_display_name(
        &self,
        id: UserId,
        display_name: &str,
    ) -> Result<(), UserRepoError>;
    /// Update the user's email. Returns [`UserRepoError::DuplicateEmail`] if
    /// another row already holds the address.
    async fn update_email(&self, id: UserId, new_email: &str) -> Result<(), UserRepoError>;
    /// Clears `email_verified_at`. Used after `update_email` so the new
    /// address must be re-verified via the standard magic-link flow.
    async fn mark_email_unverified(&self, id: UserId) -> Result<(), UserRepoError>;

    /// Set (or clear) the object-storage key for this user's avatar.
    /// Returns the PREVIOUS key so the caller can best-effort-delete the
    /// orphaned bytes from the store after the DB commit lands.
    async fn set_avatar_key(
        &self,
        id: UserId,
        avatar_key: Option<String>,
    ) -> Result<Option<String>, UserRepoError>;
}

#[cfg(test)]
mod tests {
    use super::EdgeStyle;

    #[test]
    fn edge_style_parse_round_trips_both_variants() {
        for style in [EdgeStyle::Curved, EdgeStyle::Angular] {
            assert_eq!(EdgeStyle::parse(style.as_str()), Some(style));
        }
    }

    #[test]
    fn edge_style_parse_rejects_unknown_and_wrong_case() {
        assert_eq!(EdgeStyle::parse("zigzag"), None);
        assert_eq!(EdgeStyle::parse("Angular"), None);
        assert_eq!(EdgeStyle::parse(""), None);
    }
}
