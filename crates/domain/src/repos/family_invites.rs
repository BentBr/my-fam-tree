use async_trait::async_trait;
use time::OffsetDateTime;
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
    pub expires_at: OffsetDateTime,
    pub accepted_at: Option<OffsetDateTime>,
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
        expires_at: OffsetDateTime,
    ) -> Result<Uuid, InviteRepoError>;

    /// Non-mutating lookup of a pending invite by token hash.
    ///
    /// Unlike [`accept`](Self::accept) this does NOT mark `accepted_at` — it
    /// lets the API validate the acting user (session email match / user
    /// resolution) BEFORE the single-use token is burned. Returns
    /// [`InviteRepoError::NotFoundOrAccepted`] when no unaccepted row matches
    /// and [`InviteRepoError::Expired`] when the matched row is past
    /// `expires_at`.
    async fn find_pending(
        &self,
        token_hash: &[u8],
        now: OffsetDateTime,
    ) -> Result<Invite, InviteRepoError>;

    /// Atomic accept: marks `accepted_at` and returns the invite if not already accepted and
    /// not expired.
    async fn accept(
        &self,
        token_hash: &[u8],
        now: OffsetDateTime,
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
