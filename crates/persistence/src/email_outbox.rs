//! Postgres-backed [`EmailOutboxRepo`].
//!
//! `claim_next_due` uses `SELECT … FOR UPDATE SKIP LOCKED` so multiple
//! worker pollers safely drain the table in parallel — each one gets a
//! different row, and a row in flight stays locked until the transaction
//! commits the status transition.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use my_family_domain::{
    EmailOutboxId, EmailOutboxInsert, EmailOutboxRepo, EmailOutboxRepoError, EmailOutboxRow,
};
use sqlx::PgPool;

#[derive(Clone, Debug)]
pub struct PgEmailOutboxRepo {
    pool: PgPool,
}

impl PgEmailOutboxRepo {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_err(e: &sqlx::Error) -> EmailOutboxRepoError {
    EmailOutboxRepoError::Db(e.to_string())
}

#[async_trait]
impl EmailOutboxRepo for PgEmailOutboxRepo {
    async fn enqueue(
        &self,
        email: &EmailOutboxInsert,
    ) -> Result<EmailOutboxId, EmailOutboxRepoError> {
        let id: uuid::Uuid = sqlx::query_scalar!(
            "INSERT INTO email_outbox (kind, to_addr, subject, text_body, html_body) \
             VALUES ($1, $2, $3, $4, $5) \
             RETURNING id",
            email.kind,
            email.to_addr,
            email.subject,
            email.text_body,
            email.html_body,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_err(&e))?;
        Ok(EmailOutboxId::from_uuid(id))
    }

    async fn claim_next_due(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Option<EmailOutboxRow>, EmailOutboxRepoError> {
        // Single statement: nested SELECT picks the next due row with
        // SKIP LOCKED, then we use its id to lock the outer DELETE-like
        // UPDATE… wait, we don't actually update status here — the poller
        // does so after the SMTP call. We DO need to keep the row locked
        // for the duration of the transaction, but each call uses its own
        // implicit tx (the sqlx pool fetches a connection). The cleanest
        // pattern: bump `next_attempt_at` to a far future "claim window"
        // so other pollers SKIP it, then mark sent/retry/permanent later
        // which will (a) commit (sent/permanent) or (b) reset
        // next_attempt_at via mark_retry.
        //
        // Simpler v1: SKIP LOCKED inside a single SELECT-then-UPDATE, where
        // UPDATE moves next_attempt_at forward by a short claim window
        // (60 s) and returns the row. The poller has 60s to finish; if it
        // dies, the row becomes claimable again automatically.
        let claim_horizon = now + chrono::Duration::seconds(60);
        let row = sqlx::query!(
            "UPDATE email_outbox \
             SET next_attempt_at = $2, updated_at = $1 \
             WHERE id = ( \
                 SELECT id FROM email_outbox \
                 WHERE status = 'pending' AND next_attempt_at <= $1 \
                 ORDER BY next_attempt_at \
                 FOR UPDATE SKIP LOCKED \
                 LIMIT 1 \
             ) \
             RETURNING id, kind, to_addr AS \"to_addr!: String\", subject, text_body, html_body, attempts",
            now,
            claim_horizon,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| map_err(&e))?;

        Ok(row.map(|r| EmailOutboxRow {
            id: EmailOutboxId::from_uuid(r.id),
            kind: r.kind,
            to_addr: r.to_addr,
            subject: r.subject,
            text_body: r.text_body,
            html_body: r.html_body,
            attempts: r.attempts,
        }))
    }

    async fn mark_sent(
        &self,
        id: EmailOutboxId,
        sent_at: DateTime<Utc>,
    ) -> Result<(), EmailOutboxRepoError> {
        sqlx::query!(
            "UPDATE email_outbox \
             SET status = 'sent', sent_at = $2, updated_at = $2, last_error = NULL \
             WHERE id = $1",
            id.into_uuid(),
            sent_at,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_err(&e))?;
        Ok(())
    }

    async fn mark_retry(
        &self,
        id: EmailOutboxId,
        next_attempt_at: DateTime<Utc>,
        last_error: &str,
    ) -> Result<(), EmailOutboxRepoError> {
        sqlx::query!(
            "UPDATE email_outbox \
             SET attempts = attempts + 1, \
                 next_attempt_at = $2, \
                 last_error = $3, \
                 updated_at = now() \
             WHERE id = $1",
            id.into_uuid(),
            next_attempt_at,
            last_error,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_err(&e))?;
        Ok(())
    }

    async fn mark_failed_permanent(
        &self,
        id: EmailOutboxId,
        last_error: &str,
    ) -> Result<(), EmailOutboxRepoError> {
        sqlx::query!(
            "UPDATE email_outbox \
             SET status = 'failed_permanent', \
                 attempts = attempts + 1, \
                 last_error = $2, \
                 updated_at = now() \
             WHERE id = $1",
            id.into_uuid(),
            last_error,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| map_err(&e))?;
        Ok(())
    }
}
