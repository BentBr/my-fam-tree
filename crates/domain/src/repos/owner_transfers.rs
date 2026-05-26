//! Owner-transfer repository.
//!
//! Phase E - two-token ownership handoff. A family may have at most one
//! pending transfer at any time (enforced by the partial unique index on
//! `family_owner_transfers`). The state machine is:
//!
//! - `begin` -> row inserted with both token hashes, both `*_confirmed_at`
//!   NULL, `expires_at = now + 1h`, `completed_at` / `cancelled_at` NULL.
//! - `confirm` -> looks up the row by either token hash and sets the
//!   matching side's `*_confirmed_at`. Returns the row + which side was
//!   confirmed.
//! - `complete` -> writes `completed_at`; called by the API after the role
//!   swap commits.
//! - `cancel` -> writes `cancelled_at`; owner-only.
//! - `find_active` -> read the current pending transfer (if any).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{FamilyId, UserId};

/// A row of the `family_owner_transfers` table.
#[derive(Debug, Clone)]
pub struct OwnerTransfer {
    pub id: Uuid,
    pub family_id: FamilyId,
    pub from_user_id: UserId,
    pub to_user_id: UserId,
    pub from_confirmed_at: Option<DateTime<Utc>>,
    pub to_confirmed_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, thiserror::Error)]
pub enum OwnerTransferRepoError {
    #[error("database: {0}")]
    Db(String),
    #[error("a transfer is already pending for this family")]
    AlreadyPending,
    #[error("no active transfer matches the supplied token")]
    NotFound,
    #[error("transfer has expired")]
    Expired,
}

/// Which side of a transfer a confirmation token belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferSide {
    From,
    To,
}

impl TransferSide {
    /// Lower-case string label used in audit metadata.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::From => "from",
            Self::To => "to",
        }
    }
}

#[async_trait]
pub trait OwnerTransferRepo: Send + Sync {
    /// Create a new pending transfer. Fails with `AlreadyPending` if one
    /// exists for the same family (the partial unique index enforces it
    /// at the DB level too).
    async fn begin(
        &self,
        family_id: FamilyId,
        from_user_id: UserId,
        to_user_id: UserId,
        from_token_hash: &[u8],
        to_token_hash: &[u8],
        expires_at: DateTime<Utc>,
    ) -> Result<Uuid, OwnerTransferRepoError>;

    /// Mark one side of a transfer as confirmed.
    ///
    /// Looks up the active transfer by token hash, sets the relevant
    /// `*_confirmed_at`, and returns the full row + which side was
    /// confirmed. Returns `NotFound` if no active transfer matches.
    async fn confirm(
        &self,
        token_hash: &[u8],
        now: DateTime<Utc>,
    ) -> Result<(OwnerTransfer, TransferSide), OwnerTransferRepoError>;

    /// Mark a transfer completed; meant to be called by the API after
    /// the role swap has been committed. Idempotent.
    async fn complete(
        &self,
        id: Uuid,
        now: DateTime<Utc>,
    ) -> Result<(), OwnerTransferRepoError>;

    /// Cancel the active pending transfer for a family (owner-only).
    async fn cancel(
        &self,
        family_id: FamilyId,
        now: DateTime<Utc>,
    ) -> Result<(), OwnerTransferRepoError>;

    /// Read the active transfer for the family, if any.
    async fn find_active(
        &self,
        family_id: FamilyId,
    ) -> Result<Option<OwnerTransfer>, OwnerTransferRepoError>;
}
