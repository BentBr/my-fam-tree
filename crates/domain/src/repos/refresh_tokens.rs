use std::net::IpAddr;

use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::UserId;

#[derive(Debug, Clone)]
pub struct RefreshTokenRecord {
    pub id: Uuid,
    pub user_id: UserId,
    pub created_at: OffsetDateTime,
    pub last_used_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
    pub absolute_expires_at: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
    pub device_label: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RefreshRepoError {
    #[error("database: {0}")]
    Db(String),
    #[error("not found")]
    NotFound,
}

#[async_trait]
#[allow(clippy::too_many_arguments)] // create/rotate carry full audit context (UA, IP, device, deadlines).
pub trait RefreshTokenRepo: Send + Sync {
    async fn create(
        &self,
        user_id: UserId,
        token_hash: &[u8],
        device_label: Option<&str>,
        user_agent: Option<&str>,
        ip: Option<IpAddr>,
        rolling_expires_at: OffsetDateTime,
        absolute_expires_at: OffsetDateTime,
    ) -> Result<Uuid, RefreshRepoError>;

    /// Returns the record if found and not revoked. Caller compares `expires_at` and
    /// `absolute_expires_at`.
    async fn find_active_by_hash(
        &self,
        token_hash: &[u8],
    ) -> Result<Option<RefreshTokenRecord>, RefreshRepoError>;

    /// Returns the record by hash REGARDLESS of revocation state (the
    /// returned [`RefreshTokenRecord`] carries `revoked_at`). Used for refresh
    /// token reuse detection: when an already-rotated (revoked) token is
    /// re-presented, the caller can detect theft and revoke every session for
    /// the user.
    async fn find_any_by_hash(
        &self,
        token_hash: &[u8],
    ) -> Result<Option<RefreshTokenRecord>, RefreshRepoError>;

    /// Atomic rotation: revokes the old row and inserts a new one for the same user.
    async fn rotate(
        &self,
        old_hash: &[u8],
        new_hash: &[u8],
        new_rolling_expires_at: OffsetDateTime,
        device_label: Option<&str>,
        user_agent: Option<&str>,
        ip: Option<IpAddr>,
    ) -> Result<(), RefreshRepoError>;

    async fn revoke_by_hash(&self, token_hash: &[u8]) -> Result<(), RefreshRepoError>;
    async fn revoke_all_for_user(&self, user_id: UserId) -> Result<u64, RefreshRepoError>;
    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<RefreshTokenRecord>, RefreshRepoError>;
}
