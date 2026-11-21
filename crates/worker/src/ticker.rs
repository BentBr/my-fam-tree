//! The scheduling tick.
//!
//! For each user with reminders enabled, if their local time is in the 06:00
//! hour, schedule today's digest (idempotently) and enqueue it. Safe to re-run
//! within the hour.

use my_fam_tree_cache::ReminderJob;
use my_fam_tree_domain::dates::{DEFAULT_TIMEZONE, DIGEST_LOCAL_HOUR};
use time::Duration;

use crate::digest::events_for_user_on;
use crate::state::WorkerState;

/// Run one tick. Returns the number of digests freshly scheduled (for logging
/// + tests).
///
/// Which users are "in their 06:00 hour" (and their local date) is resolved
/// by the database from each user's IANA timezone — see
/// `ReminderPreferencesRepo::due_at_local_hour`.
///
/// # Errors
/// Propagates repo / queue errors.
pub async fn run_tick(state: &WorkerState) -> anyhow::Result<usize> {
    let now_utc = state.clock.now();
    let mut scheduled = 0_usize;
    let due = state.prefs.due_at_local_hour(now_utc, DIGEST_LOCAL_HOUR, DEFAULT_TIMEZONE).await?;
    for user in due {
        let user_id = user.user_id;
        let today_local = user.local_date;
        let prefs = state.prefs.get(user_id).await?;
        let Some(target_date) = today_local.checked_add(Duration::days(i64::from(prefs.lead_days)))
        else {
            continue;
        };

        let events = events_for_user_on(state, user_id, &prefs, today_local, target_date).await?;
        if events.is_empty() {
            continue;
        }
        let count = i32::try_from(events.len()).unwrap_or(i32::MAX);
        let (digest_id, inserted) =
            state.digests.ensure_pending(user_id, today_local, count).await?;
        if inserted {
            state.queue.push(&ReminderJob { digest_id }).await?;
            scheduled = scheduled.saturating_add(1);
        }
    }
    Ok(scheduled)
}
