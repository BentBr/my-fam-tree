//! Digest dispatcher.
//!
//! Drains the queue: load the digest, re-project its events, render + send one
//! email, then mark sent or schedule a retry.

use std::time::Duration;

use my_family_cache::ReminderJob;
use my_family_email::{Locale, OutboundEmail, ReminderDigestArgs, render_reminder_digest};

use crate::backoff::next_attempt;
use crate::digest::{events_for_user_on, render_line};
use crate::state::WorkerState;

/// Poll interval when the queue is empty.
const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Long-running dispatcher loop. Polls the queue; on a job, handles it; on
/// empty, sleeps `POLL_INTERVAL`.
pub async fn run_dispatcher(state: WorkerState) {
    loop {
        match state.queue.try_pop().await {
            Ok(Some(job)) => {
                let digest_id = job.digest_id;
                if let Err(e) = handle(&state, &job).await {
                    tracing::error!(?e, %digest_id, "digest dispatch error");
                }
            }
            Ok(None) => tokio::time::sleep(POLL_INTERVAL).await,
            Err(e) => {
                tracing::error!(?e, "queue pop failed; backing off");
                tokio::time::sleep(POLL_INTERVAL).await;
            }
        }
    }
}

/// Handle one queued digest. Public so integration tests can drive a single
/// job deterministically without spawning the loop.
///
/// # Errors
/// Propagates repo / queue / email-template errors. SMTP failures are NOT
/// errors here — they're recorded on the digest row and (maybe) re-queued.
pub async fn handle(state: &WorkerState, job: &ReminderJob) -> anyhow::Result<()> {
    let Some(digest) = state.digests.find_by_id(job.digest_id).await? else { return Ok(()) };
    let Some(user) = state.users.find_by_id(digest.user_id).await? else { return Ok(()) };
    let prefs = state.prefs.get(digest.user_id).await?;

    // Re-project at send time so edits between scheduling + sending are
    // reflected. `today` is the digest's send_date; the window is
    // send_date + lead_days.
    let today = digest.send_date;
    let target = today + chrono::Duration::days(i64::from(prefs.lead_days));
    let events = events_for_user_on(state, digest.user_id, &prefs, today, target).await?;

    if events.is_empty() {
        // Everything got edited/deleted away — nothing to send.
        state.digests.mark_sent(digest.id).await?;
        return Ok(());
    }

    let locale = Locale::from_str_or_en(user.locale.as_str());
    let lines: Vec<String> = events.iter().map(|e| render_line(locale, e)).collect();
    let tree_link = format!("{}/tree", state.web_public_url);
    let manage_link = format!("{}/account", state.web_public_url);

    let (subject, body) = render_reminder_digest(
        locale,
        &ReminderDigestArgs {
            lead_days: prefs.lead_days,
            lines: &lines,
            tree_link: &tree_link,
            manage_link: &manage_link,
        },
    )?;

    let send = state
        .email
        .send(OutboundEmail {
            to_addr: user.email.clone(),
            to_name: None,
            subject,
            text_body: body,
            html_body: None,
        })
        .await;

    match send {
        Ok(()) => state.digests.mark_sent(digest.id).await?,
        Err(e) => {
            let now = state.clock.now();
            let next_at = if digest.attempt_count >= state.max_retries {
                None
            } else {
                Some(next_attempt(
                    now,
                    digest.attempt_count + 1,
                    state.retry_min_seconds,
                    state.retry_max_seconds,
                ))
            };
            state.digests.mark_failed_or_retry(digest.id, &e.to_string(), next_at).await?;
            if next_at.is_some() {
                state.queue.push(&ReminderJob { digest_id: digest.id }).await?;
            }
        }
    }
    Ok(())
}
