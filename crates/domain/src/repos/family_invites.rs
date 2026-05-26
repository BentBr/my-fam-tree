use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{FamilyId, Role, UserId};

#[derive(Debug, Clone)]
pub struct Invite {
    pub id: Uuid,
    pub family_id: FamilyId,
    pub email: String,
    pub invited_role: Role,
    pub invited_by: UserId,
    pub person_id: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, thiserror::Error)]
pub enum InviteRepoError {
    #[error("database: {0}")]
    Db(String),
    #[error("not found or already accepted")]
    NotFoundOrAccepted,
    #[error("expired")]
    Expired,
}

#[async_trait]
pub trait FamilyInviteRepo: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn create(
        &self,
        family_id: FamilyId,
        email: &str,
        invited_role: Role,
        invited_by: UserId,
        person_id: Option<Uuid>,
        token_hash: &[u8],
        expires_at: DateTime<Utc>,
    ) -> Result<Uuid, InviteRepoError>;

    /// Atomic accept: marks `accepted_at` and returns the invite if not already accepted and
    /// not expired.
    async fn accept(
        &self,
        token_hash: &[u8],
        now: DateTime<Utc>,
    ) -> Result<Invite, InviteRepoError>;

    async fn list_pending_for_family(
        &self,
        family_id: FamilyId,
    ) -> Result<Vec<Invite>, InviteRepoError>;

    /// Lookup a pending (not-accepted, not-expired) invite by email within
    /// the given family. Returns `None` when no such row exists, so the API
    /// layer can return `ApiError::InviteDuplicate` only when truthful.
    async fn find_pending_by_email(
        &self,
        family_id: FamilyId,
        email: &str,
    ) -> Result<Option<Invite>, InviteRepoError>;

    async fn cancel(&self, id: Uuid, family_id: FamilyId) -> Result<(), InviteRepoError>;
}
