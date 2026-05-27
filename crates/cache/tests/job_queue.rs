//! Integration test for the reminder digest queue.
//!
//! Requires `REDIS_URL` (compose service `redis`). Silently no-ops when unset
//! so the suite stays green locally without infrastructure; CI runs redis as a
//! service. Each test uses a unique key prefix so parallel runs don't collide.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use my_family_cache::{RedisPool, RedisReminderQueue, ReminderJob, ReminderJobQueue};
use uuid::Uuid;

fn queue() -> Option<RedisReminderQueue> {
    let url = std::env::var("REDIS_URL").ok()?;
    let prefix = format!("jq-test-{}:", Uuid::new_v4());
    let pool = RedisPool::build(&url, 4, prefix).expect("build pool");
    Some(RedisReminderQueue::new(pool))
}

#[tokio::test]
async fn push_then_pop_round_trips_the_job() {
    let Some(q) = queue() else { return };
    let job = ReminderJob { digest_id: Uuid::new_v4() };
    q.push(&job).await.unwrap();
    assert_eq!(q.try_pop().await.unwrap(), Some(job));
}

#[tokio::test]
async fn try_pop_returns_none_when_empty() {
    let Some(q) = queue() else { return };
    assert_eq!(q.try_pop().await.unwrap(), None);
}

#[tokio::test]
async fn fifo_order_two_jobs() {
    let Some(q) = queue() else { return };
    let a = ReminderJob { digest_id: Uuid::new_v4() };
    let b = ReminderJob { digest_id: Uuid::new_v4() };
    q.push(&a).await.unwrap();
    q.push(&b).await.unwrap();
    // LPUSH + RPOP ⇒ FIFO: first pushed is first popped.
    assert_eq!(q.try_pop().await.unwrap(), Some(a));
    assert_eq!(q.try_pop().await.unwrap(), Some(b));
    assert_eq!(q.try_pop().await.unwrap(), None);
}
