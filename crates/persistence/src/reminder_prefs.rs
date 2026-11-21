//! Postgres-backed [`ReminderPreferencesRepo`].
//!
//! One row per user (`reminder_preferences`, migration 0009). `get` returns
//! built-in defaults when absent so the API never has to special-case a
//! first-time user; `upsert` is an `INSERT ... ON CONFLICT (user_id) DO UPDATE`.

use async_trait::async_trait;
use my_fam_tree_domain::{
    DueDigestUser, ReminderPreferences, ReminderPreferencesRepo, ReminderPrefsRepoError, UserId,
};
use sqlx::PgPool;
use time::OffsetDateTime;

#[derive(Clone, Debug)]
pub struct PgReminderPrefsRepo {
    pool: PgPool,
}

impl PgReminderPrefsRepo {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReminderPreferencesRepo for PgReminderPrefsRepo {
    async fn get(&self, user_id: UserId) -> Result<ReminderPreferences, ReminderPrefsRepoError> {
        let row = sqlx::query!(
            r#"SELECT emails_enabled, remind_birthdays, remind_anniversaries,
                      favourites_only, lead_days
                 FROM reminder_preferences WHERE user_id = $1"#,
            user_id.into_uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ReminderPrefsRepoError::Db(e.to_string()))?;
        Ok(row.map_or_else(ReminderPreferences::default, |r| ReminderPreferences {
            emails_enabled: r.emails_enabled,
            remind_birthdays: r.remind_birthdays,
            remind_anniversaries: r.remind_anniversaries,
            favourites_only: r.favourites_only,
            lead_days: r.lead_days,
        }))
    }

    async fn upsert(
        &self,
        user_id: UserId,
        prefs: ReminderPreferences,
    ) -> Result<(), ReminderPrefsRepoError> {
        sqlx::query!(
            r#"INSERT INTO reminder_preferences
                   (user_id, emails_enabled, remind_birthdays, remind_anniversaries,
                    favourites_only, lead_days)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT (user_id) DO UPDATE SET
                   emails_enabled = EXCLUDED.emails_enabled,
                   remind_birthdays = EXCLUDED.remind_birthdays,
                   remind_anniversaries = EXCLUDED.remind_anniversaries,
                   favourites_only = EXCLUDED.favourites_only,
                   lead_days = EXCLUDED.lead_days"#,
            user_id.into_uuid(),
            prefs.emails_enabled,
            prefs.remind_birthdays,
            prefs.remind_anniversaries,
            prefs.favourites_only,
            prefs.lead_days,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ReminderPrefsRepoError::Db(e.to_string()))?;
        Ok(())
    }

    async fn due_at_local_hour(
        &self,
        now: OffsetDateTime,
        local_hour: u8,
        fallback_tz: &str,
    ) -> Result<Vec<DueDigestUser>, ReminderPrefsRepoError> {
        // `pg_timezone_names` reads the tz catalogue from disk; as a join
        // input it is scanned once per query (not per user). A stored name it
        // doesn't contain (typo, legacy value) falls back to $3.
        let rows = sqlx::query!(
            r#"WITH zones AS (SELECT name FROM pg_timezone_names)
               SELECT rp.user_id AS "user_id!",
                      (($1::timestamptz) AT TIME ZONE COALESCE(z.name, $3::text))::date
                          AS "local_date!"
                 FROM reminder_preferences rp
                 JOIN users u ON u.id = rp.user_id
            LEFT JOIN zones z ON z.name = u.timezone
                WHERE rp.emails_enabled = true
                  AND EXTRACT(HOUR FROM ($1::timestamptz) AT TIME ZONE COALESCE(z.name, $3::text))::int4
                      = $2::int4
             ORDER BY rp.user_id"#,
            now,
            i32::from(local_hour),
            fallback_tz,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ReminderPrefsRepoError::Db(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|r| DueDigestUser {
                user_id: UserId::from_uuid(r.user_id),
                local_date: r.local_date,
            })
            .collect())
    }
}
