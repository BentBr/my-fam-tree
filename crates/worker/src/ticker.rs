//! The scheduling tick.
//!
//! For each user with reminders enabled, if their local time is in the 06:00
//! hour, schedule today's digest (idempotently) and enqueue it. Safe to re-run
//! within the hour.

use std::str::FromStr;

use chrono::{Duration, Timelike};
use chrono_tz::Tz;
use my_fam_tree_cache::ReminderJob;

use crate::digest::events_for_user_on;
use crate::state::WorkerState;

/// Run one tick. Returns the number of digests freshly scheduled (for logging
/// + tests).
///
/// # Errors
/// Propagates repo / queue errors.
pub async fn run_tick(state: &WorkerState) -> anyhow::Result<usize> {
    let now_utc = state.clock.now();
    let mut scheduled = 0_usize;
    for user_id in state.prefs.enabled_user_ids().await? {
        let Some(user) = state.users.find_by_id(user_id).await? else { continue };
        let tz: Tz = Tz::from_str(&user.timezone).unwrap_or(chrono_tz::Europe::Berlin);
        let local = now_utc.with_timezone(&tz);
        if local.hour() != 6 {
            continue;
        }
        let today_local = local.date_naive();
        let prefs = state.prefs.get(user_id).await?;
        let target_date = today_local + Duration::days(i64::from(prefs.lead_days));

        let events = events_for_user_on(state, user_id, &prefs, today_local, target_date).await?;
        if events.is_empty() {
            continue;
        }
        let count = i32::try_from(events.len()).unwrap_or(i32::MAX);
        let (digest_id, inserted) =
            state.digests.ensure_pending(user_id, today_local, count).await?;
        if inserted {
            state.queue.push(&ReminderJob { digest_id }).await?;
            scheduled += 1;
        }
    }
    Ok(scheduled)
}
