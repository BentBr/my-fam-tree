//! `EmailOutboxRepo` — durable transactional-email outbox.
//!
//! Producers (the API handlers that previously called `EmailSender::send()`
//! directly) insert pre-rendered emails here in the same Postgres transaction
//! as the user-visible side effect. The worker's outbox poller claims due
//! rows via `SELECT … FOR UPDATE SKIP LOCKED` so multiple dispatcher tasks
//! drain in parallel without seeing the same row, sends via SMTP, and marks
//! the row sent / retry / permanent-failure.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Newtype around the outbox row id. Display + serde transparent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmailOutboxId(Uuid);

impl EmailOutboxId {
    #[must_use]
    pub const fn from_uuid(u: Uuid) -> Self {
        Self(u)
    }
    #[must_use]
    pub const fn into_uuid(self) -> Uuid {
        self.0
    }
}

/// Logical kind of email. TEXT in the DB so new kinds don't need a
/// migration; the worker treats it as an opaque label.
#[derive(Debug, Clone)]
pub struct EmailOutboxKind(pub String);

impl EmailOutboxKind {
    pub const MAGIC_LINK: &'static str = "magic_link";
    pub const INVITE: &'static str = "invite";
    pub const OWNER_TRANSFER_FROM: &'static str = "owner_transfer_from";
    pub const OWNER_TRANSFER_TO: &'static str = "owner_transfer_to";
    pub const EMAIL_CHANGE: &'static str = "email_change";
    pub const REMINDER_DIGEST: &'static str = "reminder_digest";
}

/// Producer payload: the email is fully rendered (subject/body), so the
/// worker just SMTPs it — no locale/template machinery in the worker path.
#[derive(Debug, Clone)]
pub struct EmailOutboxInsert {
    pub kind: String,
    pub to_addr: String,
    pub subject: String,
    pub text_body: String,
    pub html_body: Option<String>,
}

/// A claimed (pending, due) outbox row, ready for SMTP.
#[derive(Debug, Clone)]
pub struct EmailOutboxRow {
    pub id: EmailOutboxId,
    pub kind: String,
    pub to_addr: String,
    pub subject: String,
    pub text_body: String,
    pub html_body: Option<String>,
    pub attempts: i32,
}

#[derive(Debug, thiserror::Error)]
pub enum EmailOutboxRepoError {
    #[error("database: {0}")]
    Db(String),
}

#[async_trait]
pub trait EmailOutboxRepo: Send + Sync + 'static {
    /// Insert a pending email. The row is ready for the next worker poll
    /// (`next_attempt_at = now()` by default).
    async fn enqueue(
        &self,
        email: &EmailOutboxInsert,
    ) -> Result<EmailOutboxId, EmailOutboxRepoError>;

    /// Claim ONE due pending row for sending. Uses `SELECT … FOR UPDATE
    /// SKIP LOCKED` so parallel pollers can drain without contention.
    /// Returns `None` if nothing is due.
    async fn claim_next_due(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Option<EmailOutboxRow>, EmailOutboxRepoError>;

    /// Terminal: SMTP succeeded.
    async fn mark_sent(
        &self,
        id: EmailOutboxId,
        sent_at: DateTime<Utc>,
    ) -> Result<(), EmailOutboxRepoError>;

    /// Non-terminal: SMTP failed, but the row still has retries left.
    /// Schedules the next attempt and records the last error message.
    async fn mark_retry(
        &self,
        id: EmailOutboxId,
        next_attempt_at: DateTime<Utc>,
        last_error: &str,
    ) -> Result<(), EmailOutboxRepoError>;

    /// Terminal: too many failures (or a non-retryable error).
    async fn mark_failed_permanent(
        &self,
        id: EmailOutboxId,
        last_error: &str,
    ) -> Result<(), EmailOutboxRepoError>;
}
