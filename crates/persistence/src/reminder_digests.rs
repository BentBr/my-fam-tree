//! Postgres-backed [`ReminderDigestRepo`] (`reminder_digests`, migration 0009).

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use my_fam_tree_domain::{
    DigestRepoError, DigestStatus, ReminderDigest, ReminderDigestRepo, UserId,
};
use sqlx::PgPool;
use uuid::Uuid;

fn status_from(s: &str) -> DigestStatus {
    match s {
        "sent" => DigestStatus::Sent,
        "failed" => DigestStatus::Failed,
        _ => DigestStatus::Pending,
    }
}

#[derive(Clone, Debug)]
pub struct PgReminderDigestRepo {
    pool: PgPool,
}

impl PgReminderDigestRepo {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReminderDigestRepo for PgReminderDigestRepo {
    async fn ensure_pending(
        &self,
        user_id: UserId,
        send_date: NaiveDate,
        event_count: i32,
    ) -> Result<(Uuid, bool), DigestRepoError> {
        let inserted = sqlx::query!(
            r#"INSERT INTO reminder_digests (user_id, send_date, event_count)
               VALUES ($1, $2, $3)
               ON CONFLICT (user_id, send_date) DO NOTHING
               RETURNING id"#,
            user_id.into_uuid(),
            send_date,
            event_count,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DigestRepoError::Db(e.to_string()))?;

        if let Some(r) = inserted {
            return Ok((r.id, true));
        }
        let existing = sqlx::query!(
            "SELECT id FROM reminder_digests WHERE user_id = $1 AND send_date = $2",
            user_id.into_uuid(),
            send_date,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DigestRepoError::Db(e.to_string()))?;
        Ok((existing.id, false))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<ReminderDigest>, DigestRepoError> {
        let row = sqlx::query!(
            r#"SELECT id, user_id, send_date, event_count, status::text as "status!",
                      error, attempt_count, next_attempt_at, dispatched_at
                 FROM reminder_digests WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DigestRepoError::Db(e.to_string()))?;
        Ok(row.map(|r| ReminderDigest {
            id: r.id,
            user_id: UserId::from_uuid(r.user_id),
            send_date: r.send_date,
            event_count: r.event_count,
            status: status_from(&r.status),
            error: r.error,
            attempt_count: r.attempt_count,
            next_attempt_at: r.next_attempt_at,
            dispatched_at: r.dispatched_at,
        }))
    }

    async fn mark_sent(&self, id: Uuid) -> Result<(), DigestRepoError> {
        sqlx::query!(
            "UPDATE reminder_digests SET status = 'sent'::reminder_status, dispatched_at = now(), error = '' WHERE id = $1",
            id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| DigestRepoError::Db(e.to_string()))?;
        Ok(())
    }

    #[allow(
        clippy::option_if_let_else,
        reason = "the two arms run distinct SQL statements; map_or_else over query! macros is less readable"
    )]
    async fn mark_failed_or_retry(
        &self,
        id: Uuid,
        error: &str,
        next_attempt_at: Option<DateTime<Utc>>,
    ) -> Result<(), DigestRepoError> {
        match next_attempt_at {
            Some(when) => sqlx::query!(
                "UPDATE reminder_digests SET error = $2, attempt_count = attempt_count + 1, next_attempt_at = $3 WHERE id = $1",
                id,
                error,
                when
            ),
            None => sqlx::query!(
                "UPDATE reminder_digests SET status = 'failed'::reminder_status, error = $2, attempt_count = attempt_count + 1 WHERE id = $1",
                id,
                error
            ),
        }
        .execute(&self.pool)
        .await
        .map_err(|e| DigestRepoError::Db(e.to_string()))?;
        Ok(())
    }

    async fn list_for_user(
        &self,
        user_id: UserId,
        limit: i64,
    ) -> Result<Vec<ReminderDigest>, DigestRepoError> {
        let rows = sqlx::query!(
            r#"SELECT id, user_id, send_date, event_count, status::text as "status!",
                      error, attempt_count, next_attempt_at, dispatched_at
                 FROM reminder_digests
                WHERE user_id = $1
                ORDER BY send_date DESC
                LIMIT $2"#,
            user_id.into_uuid(),
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DigestRepoError::Db(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|r| ReminderDigest {
                id: r.id,
                user_id: UserId::from_uuid(r.user_id),
                send_date: r.send_date,
                event_count: r.event_count,
                status: status_from(&r.status),
                error: r.error,
                attempt_count: r.attempt_count,
                next_attempt_at: r.next_attempt_at,
                dispatched_at: r.dispatched_at,
            })
            .collect())
    }
}
