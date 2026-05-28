//! `JanitorRepo` — periodic cleanup of expired auth/invite/transfer rows.
//!
//! The worker hosts a low-frequency tick (default every 5 min) that calls
//! [`JanitorRepo::sweep_expired`] under the leader lock so exactly one
//! instance runs the deletes at a time. Each call returns per-table counts
//! for structured logging — the sweep is silent at INFO when nothing was
//! deleted and chatty when it actually does work.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};

/// Number of rows deleted by a single [`JanitorRepo::sweep_expired`] call.
#[derive(Debug, Clone, Default)]
pub struct JanitorSweepReport {
    pub magic_links_deleted: u64,
    pub refresh_tokens_deleted: u64,
    pub family_invites_deleted: u64,
    pub owner_transfers_deleted: u64,
}

impl JanitorSweepReport {
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.magic_links_deleted
            + self.refresh_tokens_deleted
            + self.family_invites_deleted
            + self.owner_transfers_deleted
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JanitorRepoError {
    #[error("database: {0}")]
    Db(String),
}

#[async_trait]
pub trait JanitorRepo: Send + Sync + 'static {
    /// Delete expired / settled rows whose tombstone (`expires_at`,
    /// `consumed_at`, `revoked_at`, `completed_at`, `cancelled_at`) is older
    /// than `now - grace`. The grace window keeps recently-settled rows
    /// around for short-term debugging — they're cryptographically useless
    /// (token hashes can't be reversed) but a stale `audit` chain still
    /// reads better with the row present for a day or two.
    async fn sweep_expired(
        &self,
        now: DateTime<Utc>,
        grace: Duration,
    ) -> Result<JanitorSweepReport, JanitorRepoError>;
}
