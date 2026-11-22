//! `JanitorRepo` — periodic cleanup of expired auth/invite/transfer rows.
//!
//! The worker hosts a low-frequency tick (default every 5 min) that calls
//! [`JanitorRepo::sweep_expired`] under the leader lock so exactly one
//! instance runs the deletes at a time. Each call returns per-table counts
//! for structured logging — the sweep is silent at INFO when nothing was
//! deleted and chatty when it actually does work.

use async_trait::async_trait;
use time::{Duration, OffsetDateTime};

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
            .saturating_add(self.refresh_tokens_deleted)
            .saturating_add(self.family_invites_deleted)
            .saturating_add(self.owner_transfers_deleted)
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
        now: OffsetDateTime,
        grace: Duration,
    ) -> Result<JanitorSweepReport, JanitorRepoError>;
}

#[cfg(test)]
mod tests {
    use super::JanitorSweepReport;

    #[test]
    fn total_sums_all_tables() {
        let r = JanitorSweepReport {
            magic_links_deleted: 1,
            refresh_tokens_deleted: 2,
            family_invites_deleted: 3,
            owner_transfers_deleted: 4,
        };
        assert_eq!(r.total(), 10, "sum of the four tables");
    }

    #[test]
    fn total_saturates_instead_of_overflowing() {
        let r = JanitorSweepReport {
            magic_links_deleted: u64::MAX,
            refresh_tokens_deleted: 1,
            family_invites_deleted: 0,
            owner_transfers_deleted: 0,
        };
        assert_eq!(r.total(), u64::MAX, "saturates at u64::MAX");
    }
}
