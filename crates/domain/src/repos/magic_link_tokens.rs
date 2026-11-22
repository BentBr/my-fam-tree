use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagicLinkPurpose {
    Login,
    InviteAccept,
    EmailChange,
}

impl MagicLinkPurpose {
    #[must_use]
    pub const fn as_db(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::InviteAccept => "invite_accept",
            Self::EmailChange => "email_change",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MagicLinkRecord {
    pub id: Uuid,
    pub user_id: Option<UserId>,
    pub email: String,
    pub purpose: MagicLinkPurpose,
    pub expires_at: OffsetDateTime,
    pub consumed_at: Option<OffsetDateTime>,
}

#[derive(Debug, thiserror::Error)]
pub enum MagicLinkRepoError {
    #[error("database: {0}")]
    Db(String),
    #[error("not found or already consumed")]
    NotFoundOrConsumed,
    #[error("expired")]
    Expired,
}

#[async_trait]
pub trait MagicLinkRepo: Send + Sync {
    /// Stores a freshly issued token. `token_hash` is sha256(token).
    async fn create(
        &self,
        user_id: Option<UserId>,
        email: &str,
        token_hash: &[u8],
        purpose: MagicLinkPurpose,
        expires_at: OffsetDateTime,
    ) -> Result<Uuid, MagicLinkRepoError>;

    /// Non-mutating lookup of a consumable (not-yet-consumed) token.
    ///
    /// Unlike [`consume`](Self::consume) this does NOT mark `consumed_at` — it
    /// lets a caller validate the token's `purpose` / `user_id` against the
    /// authenticated session BEFORE the single-use token is burned. Returns
    /// [`MagicLinkRepoError::NotFoundOrConsumed`] when no unconsumed row
    /// matches and [`MagicLinkRepoError::Expired`] when the matched row is
    /// past `expires_at`.
    async fn find_consumable(
        &self,
        token_hash: &[u8],
        now: OffsetDateTime,
    ) -> Result<MagicLinkRecord, MagicLinkRepoError>;

    /// Atomically marks consumed and returns the record. Rejects with `Expired` when `expires_at < now`; `consumed_at` is set to `now`.
    async fn consume(
        &self,
        token_hash: &[u8],
        now: OffsetDateTime,
    ) -> Result<MagicLinkRecord, MagicLinkRepoError>;
}
