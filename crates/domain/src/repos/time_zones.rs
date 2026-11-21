//! IANA timezone conversions, resolved by the database.
//!
//! Postgres ships the full tz database (DST rules included), so converting
//! between UTC and a user's local wall-clock is delegated to it instead of
//! pulling a tz crate into the workspace.

use async_trait::async_trait;
use time::{Date, OffsetDateTime, Time};

/// Errors surfaced by [`TimeZoneRepo`].
#[derive(Debug, thiserror::Error)]
pub enum TimeZoneRepoError {
    /// The zone name is not in the database's tz catalogue.
    #[error("unknown time zone: {0}")]
    UnknownZone(String),
    #[error("database: {0}")]
    Db(String),
}

#[async_trait]
pub trait TimeZoneRepo: Send + Sync {
    /// The calendar date at instant `at` in zone `tz`.
    ///
    /// # Errors
    /// [`TimeZoneRepoError::UnknownZone`] for an unknown `tz`;
    /// [`TimeZoneRepoError::Db`] on query failure.
    async fn local_date(&self, at: OffsetDateTime, tz: &str) -> Result<Date, TimeZoneRepoError>;

    /// The UTC instant of wall-clock `date` + `local_time` in zone `tz`
    /// (DST-aware; a time skipped by a spring-forward gap resolves the way
    /// Postgres does, i.e. shifted forward by the gap).
    ///
    /// # Errors
    /// [`TimeZoneRepoError::UnknownZone`] for an unknown `tz`;
    /// [`TimeZoneRepoError::Db`] on query failure.
    async fn local_to_utc(
        &self,
        date: Date,
        local_time: Time,
        tz: &str,
    ) -> Result<OffsetDateTime, TimeZoneRepoError>;
}
