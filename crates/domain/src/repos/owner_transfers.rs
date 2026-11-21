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
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{FamilyId, UserId};

/// A row of the `family_owner_transfers` table.
#[derive(Debug, Clone)]
pub struct OwnerTransfer {
    pub id: Uuid,
    pub family_id: FamilyId,
    pub from_user_id: UserId,
    pub to_user_id: UserId,
    pub from_confirmed_at: Option<OffsetDateTime>,
    pub to_confirmed_at: Option<OffsetDateTime>,
    pub expires_at: OffsetDateTime,
    pub completed_at: Option<OffsetDateTime>,
    pub cancelled_at: Option<OffsetDateTime>,
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
    #[error("token does not belong to the authenticated user")]
    Forbidden,
    #[error("membership changed; transfer can no longer complete")]
    MembershipChanged,
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
        expires_at: OffsetDateTime,
    ) -> Result<Uuid, OwnerTransferRepoError>;

    /// Mark one side of a transfer as confirmed.
    ///
    /// Looks up the active transfer by token hash, determines which side the
    /// token belongs to, and REQUIRES that `actor_user_id` matches the user
    /// expected for that side (the `from_user_id` for the From side, the
    /// `to_user_id` for the To side). On mismatch it returns
    /// [`OwnerTransferRepoError::Forbidden`] BEFORE running the confirming
    /// update, so a leaked token presented by the wrong authenticated user
    /// can never burn / advance the side. On match it sets the relevant
    /// `*_confirmed_at` and returns the full row + which side was confirmed.
    /// Returns `NotFound` if no active transfer matches.
    async fn confirm(
        &self,
        token_hash: &[u8],
        actor_user_id: UserId,
        now: OffsetDateTime,
    ) -> Result<(OwnerTransfer, TransferSide), OwnerTransferRepoError>;

    /// Mark a transfer completed; meant to be called by the API after
    /// the role swap has been committed. Idempotent.
    ///
    /// **Prefer [`complete_with_role_swap`](Self::complete_with_role_swap)**
    /// for normal completion — that runs the demote + promote + complete
    /// in a single transaction so a mid-flight failure can't leave the
    /// family with no owner. This raw `complete` exists so admin tooling
    /// can mark an out-of-band swap as done without re-running the role
    /// updates.
    async fn complete(&self, id: Uuid, now: OffsetDateTime) -> Result<(), OwnerTransferRepoError>;

    /// Atomic completion: in a single SQL transaction, demote the
    /// outgoing owner to admin, promote the incoming user to owner, and
    /// stamp `completed_at` on the transfer row. Either all three land
    /// or none do — without this method the API made three separate
    /// non-transactional calls and a failure between #1 and #2 left the
    /// family with no owner row.
    ///
    /// Both role updates are guarded so the family can never be left
    /// without an owner: the demote only matches a still-current owner
    /// (`role = 'owner'`) and the promote requires the incoming
    /// membership to still exist. If either update does not affect
    /// exactly one row (e.g. the incoming user was removed/demoted
    /// between `begin` and here) the transaction rolls back and
    /// [`OwnerTransferRepoError::MembershipChanged`] is returned —
    /// nothing changes, the old owner stays owner.
    ///
    /// # Errors
    /// [`OwnerTransferRepoError::MembershipChanged`] when the demote or
    /// promote does not affect exactly one row (the transaction rolls
    /// back, so partial state never lands);
    /// [`OwnerTransferRepoError::Db`] on transport / driver failure;
    /// the transaction rolls back so partial state never lands.
    async fn complete_with_role_swap(
        &self,
        transfer_id: Uuid,
        family_id: FamilyId,
        from_user_id: UserId,
        to_user_id: UserId,
        now: OffsetDateTime,
    ) -> Result<(), OwnerTransferRepoError>;

    /// Cancel the active pending transfer for a family (owner-only).
    async fn cancel(
        &self,
        family_id: FamilyId,
        now: OffsetDateTime,
    ) -> Result<(), OwnerTransferRepoError>;

    /// Read the active transfer for the family, if any.
    async fn find_active(
        &self,
        family_id: FamilyId,
    ) -> Result<Option<OwnerTransfer>, OwnerTransferRepoError>;
}
