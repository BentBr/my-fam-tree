//! Typed Redis list used as the reminder digest queue.
//!
//! The ticker pushes a `ReminderJob { digest_id }` after scheduling a digest;
//! a pool of dispatcher tasks polls with `try_pop` and sends. We use a
//! non-blocking `RPOP` rather than `BRPOP` so a dispatcher never holds a
//! pooled connection open for the whole idle window (which would starve the
//! small connection pool and trip its response timeout). The
//! `reminder_digests` row is the source of truth, so a lost queue is
//! recoverable (Phase 5 reconcile sweep).

use async_trait::async_trait;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{CacheError, RedisPool};

/// A queued reminder digest awaiting send. The payload is just the digest row
/// id; the dispatcher loads the row + re-projects the events at send time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReminderJob {
    pub digest_id: Uuid,
}

#[async_trait]
pub trait ReminderJobQueue: Send + Sync {
    /// Enqueue a digest for sending.
    ///
    /// # Errors
    /// Returns [`CacheError`] on a connection or serialization failure.
    async fn push(&self, job: &ReminderJob) -> Result<(), CacheError>;

    /// Pop the next job without blocking; `Ok(None)` when the queue is empty.
    /// Callers poll on a sleep interval.
    ///
    /// # Errors
    /// Returns [`CacheError`] on a connection or deserialization failure.
    async fn try_pop(&self) -> Result<Option<ReminderJob>, CacheError>;
}

#[derive(Clone, Debug)]
pub struct RedisReminderQueue {
    pool: RedisPool,
}

impl RedisReminderQueue {
    #[must_use]
    pub const fn new(pool: RedisPool) -> Self {
        Self { pool }
    }

    fn key(&self) -> String {
        format!("{}queue:reminder-digest", self.pool.prefix())
    }
}

#[async_trait]
impl ReminderJobQueue for RedisReminderQueue {
    async fn push(&self, job: &ReminderJob) -> Result<(), CacheError> {
        let mut conn = self.pool.inner().get().await?;
        let payload = serde_json::to_string(job).map_err(|e| CacheError::Config(e.to_string()))?;
        // LPUSH + RPOP ⇒ FIFO.
        let _: i64 = conn.lpush(self.key(), payload).await?;
        Ok(())
    }

    async fn try_pop(&self) -> Result<Option<ReminderJob>, CacheError> {
        let mut conn = self.pool.inner().get().await?;
        let payload: Option<String> =
            redis::cmd("RPOP").arg(self.key()).query_async(&mut conn).await?;
        match payload {
            Some(p) => {
                Ok(Some(serde_json::from_str(&p).map_err(|e| CacheError::Config(e.to_string()))?))
            }
            None => Ok(None),
        }
    }
}
