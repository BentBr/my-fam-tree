use async_trait::async_trait;
use chrono::{DateTime, Utc};
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
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
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
        expires_at: DateTime<Utc>,
    ) -> Result<Uuid, MagicLinkRepoError>;

    /// Atomically marks consumed and returns the record. Caller verifies `expires_at`.
    async fn consume(&self, token_hash: &[u8]) -> Result<MagicLinkRecord, MagicLinkRepoError>;
}
