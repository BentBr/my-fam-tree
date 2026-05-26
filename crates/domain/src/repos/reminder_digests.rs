//! The digest dispatch log: one row per user per local send-date.
//!
//! Defined in migration 0009. The row is the source of truth — a flushed
//! Redis queue can be reconstructed from the pending rows here (the Phase 5
//! reconcile sweep).

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::UserId;

/// Lifecycle of a single digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestStatus {
    Pending,
    Sent,
    Failed,
}

impl DigestStatus {
    /// The Postgres `reminder_status` enum label.
    #[must_use]
    pub const fn as_db(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Sent => "sent",
            Self::Failed => "failed",
        }
    }
}

/// A scheduled (or sent / failed) daily digest.
#[derive(Debug, Clone)]
pub struct ReminderDigest {
    pub id: Uuid,
    pub user_id: UserId,
    pub send_date: NaiveDate,
    pub event_count: i32,
    pub status: DigestStatus,
    pub error: String,
    pub attempt_count: i32,
    pub next_attempt_at: DateTime<Utc>,
    pub dispatched_at: Option<DateTime<Utc>>,
}

/// Errors surfaced by [`ReminderDigestRepo`].
#[derive(Debug, thiserror::Error)]
pub enum DigestRepoError {
    #[error("database: {0}")]
    Db(String),
}

#[async_trait]
pub trait ReminderDigestRepo: Send + Sync {
    /// Idempotent insert keyed on `(user_id, send_date)`. Returns the row id
    /// (new or pre-existing) and whether it was freshly inserted. The worker
    /// only enqueues when `inserted == true`.
    ///
    /// # Errors
    /// Returns [`DigestRepoError::Db`] on query failure.
    async fn ensure_pending(
        &self,
        user_id: UserId,
        send_date: NaiveDate,
        event_count: i32,
    ) -> Result<(Uuid, bool), DigestRepoError>;

    /// Load one digest by id.
    ///
    /// # Errors
    /// Returns [`DigestRepoError::Db`] on query failure.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<ReminderDigest>, DigestRepoError>;

    /// Mark the digest sent.
    ///
    /// # Errors
    /// Returns [`DigestRepoError::Db`] on query failure.
    async fn mark_sent(&self, id: Uuid) -> Result<(), DigestRepoError>;

    /// Schedule a retry (`Some(when)`) or give up (`None` → status `failed`).
    ///
    /// # Errors
    /// Returns [`DigestRepoError::Db`] on query failure.
    async fn mark_failed_or_retry(
        &self,
        id: Uuid,
        error: &str,
        next_attempt_at: Option<DateTime<Utc>>,
    ) -> Result<(), DigestRepoError>;

    /// Recent digests for the FE history page (Phase 5 endpoint).
    ///
    /// # Errors
    /// Returns [`DigestRepoError::Db`] on query failure.
    async fn list_for_user(
        &self,
        user_id: UserId,
        limit: i64,
    ) -> Result<Vec<ReminderDigest>, DigestRepoError>;
}
