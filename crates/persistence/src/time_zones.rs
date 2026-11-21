//! Postgres-backed [`TimeZoneRepo`]: IANA zone conversions via `AT TIME ZONE`.

use async_trait::async_trait;
use my_fam_tree_domain::{TimeZoneRepo, TimeZoneRepoError};
use sqlx::PgPool;
use time::{Date, OffsetDateTime, Time};

/// SQLSTATE `22023` (`invalid_parameter_value`) — what Postgres raises for
/// `AT TIME ZONE 'Not/AZone'`.
const INVALID_PARAMETER_VALUE: &str = "22023";

#[derive(Clone, Debug)]
pub struct PgTimeZoneRepo {
    pool: PgPool,
}

impl PgTimeZoneRepo {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_err(e: &sqlx::Error, tz: &str) -> TimeZoneRepoError {
    match e.as_database_error().and_then(sqlx::error::DatabaseError::code) {
        Some(code) if code == INVALID_PARAMETER_VALUE => {
            TimeZoneRepoError::UnknownZone(tz.to_owned())
        }
        _ => TimeZoneRepoError::Db(e.to_string()),
    }
}

#[async_trait]
impl TimeZoneRepo for PgTimeZoneRepo {
    async fn local_date(&self, at: OffsetDateTime, tz: &str) -> Result<Date, TimeZoneRepoError> {
        let row = sqlx::query!(
            r#"SELECT (($1::timestamptz) AT TIME ZONE $2::text)::date AS "local_date!""#,
            at,
            tz,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_err(&e, tz))?;
        Ok(row.local_date)
    }

    async fn local_to_utc(
        &self,
        date: Date,
        local_time: Time,
        tz: &str,
    ) -> Result<OffsetDateTime, TimeZoneRepoError> {
        let row = sqlx::query!(
            r#"SELECT (($1::date + $2::time) AT TIME ZONE $3::text) AS "instant!""#,
            date,
            local_time,
            tz,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_err(&e, tz))?;
        Ok(row.instant)
    }
}
