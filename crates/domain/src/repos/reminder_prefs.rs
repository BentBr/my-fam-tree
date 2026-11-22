//! A user's reminder settings: one row per user (see migration 0009).

use async_trait::async_trait;
use time::{Date, OffsetDateTime};

use crate::UserId;

/// A user whose digest is due: reminders are on and their local wall-clock
/// is inside the digest hour. `local_date` is "today" in their zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DueDigestUser {
    pub user_id: UserId,
    pub local_date: Date,
}

/// A user's reminder settings. `lead_days` is clamped 0..=21 at the API
/// boundary and by a DB CHECK; 0 means "on the day".
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent user-facing toggles mirroring the reminder_preferences columns, not a state machine"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReminderPreferences {
    pub emails_enabled: bool,
    pub remind_birthdays: bool,
    pub remind_anniversaries: bool,
    pub favourites_only: bool,
    pub lead_days: i32,
}

impl Default for ReminderPreferences {
    /// Opt-in: emails OFF, both kinds ON, all people, 7 days ahead.
    fn default() -> Self {
        Self {
            emails_enabled: false,
            remind_birthdays: true,
            remind_anniversaries: true,
            favourites_only: false,
            lead_days: 7,
        }
    }
}

/// Errors surfaced by [`ReminderPreferencesRepo`].
#[derive(Debug, thiserror::Error)]
pub enum ReminderPrefsRepoError {
    #[error("database: {0}")]
    Db(String),
}

#[async_trait]
pub trait ReminderPreferencesRepo: Send + Sync {
    /// Current settings, or [`ReminderPreferences::default`] if the user has
    /// never saved any.
    ///
    /// # Errors
    /// Returns [`ReminderPrefsRepoError::Db`] on query failure.
    async fn get(&self, user_id: UserId) -> Result<ReminderPreferences, ReminderPrefsRepoError>;

    /// Insert-or-update the single row for this user.
    ///
    /// # Errors
    /// Returns [`ReminderPrefsRepoError::Db`] on query failure.
    async fn upsert(
        &self,
        user_id: UserId,
        prefs: ReminderPreferences,
    ) -> Result<(), ReminderPrefsRepoError>;

    /// Every user with `emails_enabled = true` whose local time at `now` is
    /// within hour `local_hour` (0–23), with their local date. The local time
    /// is resolved in the database from `users.timezone` (IANA name, DST
    /// aware); a zone name the database doesn't know falls back to
    /// `fallback_tz`. Ordered by user id for determinism.
    ///
    /// # Errors
    /// Returns [`ReminderPrefsRepoError::Db`] on query failure.
    async fn due_at_local_hour(
        &self,
        now: OffsetDateTime,
        local_hour: u8,
        fallback_tz: &str,
    ) -> Result<Vec<DueDigestUser>, ReminderPrefsRepoError>;
}
