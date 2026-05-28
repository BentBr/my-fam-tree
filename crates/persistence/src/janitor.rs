//! Postgres-backed [`JanitorRepo`] — DELETE statements for expired auth /
//! invite / owner-transfer rows. One sweep per call; the worker drives the
//! cadence (see `crates/worker/src/janitor.rs`).

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use my_family_domain::{JanitorRepo, JanitorRepoError, JanitorSweepReport};
use sqlx::PgPool;

#[derive(Clone, Debug)]
pub struct PgJanitor {
    pool: PgPool,
}

impl PgJanitor {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_err(e: &sqlx::Error) -> JanitorRepoError {
    JanitorRepoError::Db(e.to_string())
}

#[async_trait]
impl JanitorRepo for PgJanitor {
    async fn sweep_expired(
        &self,
        now: DateTime<Utc>,
        grace: Duration,
    ) -> Result<JanitorSweepReport, JanitorRepoError> {
        let cutoff = now - grace;

        // Magic-link tokens: expired OR consumed past the grace window.
        // Token hashes are useless once expired/consumed, so this is purely
        // table-hygiene — sized to keep the partial-index hot set tight.
        let m = sqlx::query!(
            "DELETE FROM magic_link_tokens \
             WHERE expires_at < $1 \
                OR (consumed_at IS NOT NULL AND consumed_at < $1)",
            cutoff,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_err(&e))?;

        // Refresh tokens: past the absolute lifetime OR revoked past grace.
        // (Sliding `expires_at` can still be inside grace even if the row is
        // useless — `absolute_expires_at` is the firm ceiling.)
        let r = sqlx::query!(
            "DELETE FROM refresh_tokens \
             WHERE absolute_expires_at < $1 \
                OR (revoked_at IS NOT NULL AND revoked_at < $1)",
            cutoff,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_err(&e))?;

        // Family invites: any row whose expires_at is past cutoff — covers
        // both never-accepted (truly stale) and accepted-and-old (the audit
        // log already captures who joined who when).
        let i = sqlx::query!("DELETE FROM family_invites WHERE expires_at < $1", cutoff,)
            .execute(&self.pool)
            .await
            .map_err(|e| map_err(&e))?;

        // Owner-transfer attempts: settled (completed/cancelled) past grace,
        // or expired without completion. The partial unique index that
        // guards "at most one pending transfer per family" already keys on
        // both NULLs so cleaning settled rows is FK-safe.
        let o = sqlx::query!(
            "DELETE FROM family_owner_transfers \
             WHERE (completed_at IS NOT NULL AND completed_at < $1) \
                OR (cancelled_at IS NOT NULL AND cancelled_at < $1) \
                OR expires_at < $1",
            cutoff,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_err(&e))?;

        Ok(JanitorSweepReport {
            magic_links_deleted: m.rows_affected(),
            refresh_tokens_deleted: r.rows_affected(),
            family_invites_deleted: i.rows_affected(),
            owner_transfers_deleted: o.rows_affected(),
        })
    }
}
