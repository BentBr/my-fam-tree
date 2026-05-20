//! Redis-backed sliding-window rate limiter.
//!
//! Implements the classic sliding-window-log algorithm using a Redis sorted
//! set per key. Each request adds an entry scored by the current millisecond
//! timestamp; entries older than the window are pruned on every call.
//!
//! The implementation issues a single pipelined round-trip:
//!
//! 1. `ZREMRANGEBYSCORE` drops entries outside the window.
//! 2. `ZADD`            inserts the current request.
//! 3. `ZCARD`           reports the current bucket size.
//! 4. `EXPIRE`          GCs the key if it falls idle.
//!
//! The decision is made client-side based on the returned count vs. the
//! caller-provided `limit`. The pipeline is *not* a `MULTI/EXEC` transaction:
//! that's deliberate — under contention the client only needs an eventually
//! correct count, and avoiding the transaction lets multiple shards share the
//! same connection multiplexer without blocking.

use std::time::Duration;

use async_trait::async_trait;

use crate::{CacheError, RedisPool};

/// Outcome of a rate-limit check.
#[derive(Debug, Clone, Copy)]
pub struct RateLimitDecision {
    /// `true` if the request is allowed under the limit.
    pub allowed: bool,
    /// The number of requests counted within the current window
    /// (including the request just recorded).
    pub count: u32,
    /// Seconds the caller should wait before retrying. `0` when allowed.
    pub retry_after_seconds: u32,
}

/// Abstract rate limiter so call-sites can be swapped with a fake in tests.
#[async_trait]
pub trait RateLimiter: Send + Sync {
    /// Records a hit for `key` and returns whether it's allowed under
    /// `limit` requests per sliding `window`.
    async fn check(
        &self,
        key: &str,
        limit: u32,
        window: Duration,
    ) -> Result<RateLimitDecision, CacheError>;
}

/// Redis-backed implementation of [`RateLimiter`].
#[derive(Clone, Debug)]
pub struct RedisRateLimiter {
    pool: RedisPool,
}

impl RedisRateLimiter {
    pub const fn new(pool: RedisPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RateLimiter for RedisRateLimiter {
    async fn check(
        &self,
        key: &str,
        limit: u32,
        window: Duration,
    ) -> Result<RateLimitDecision, CacheError> {
        let mut conn = self.pool.inner().get().await?;
        let full_key = format!("{}rate:{}", self.pool.prefix(), key);

        // Clamp the window to i64 — Redis EXPIRE takes seconds as i64.
        // Durations longer than i64::MAX seconds (~292 billion years) can't
        // realistically happen, but `try_from` keeps us lint-clean either way.
        let window_secs = i64::try_from(window.as_secs()).unwrap_or(i64::MAX);
        let now_ms = chrono::Utc::now().timestamp_millis();
        let cutoff = now_ms.saturating_sub(window_secs.saturating_mul(1_000));

        // Sorted-set member is unique per call so concurrent inserts at the
        // exact same millisecond don't collide and silently dedupe.
        let member = format!("{now_ms}-{}", uuid::Uuid::new_v4());

        let mut pipe = redis::pipe();
        pipe.zrembyscore(&full_key, i64::MIN, cutoff)
            .ignore()
            .zadd(&full_key, &member, now_ms)
            .ignore()
            .zcard(&full_key)
            .expire(&full_key, window_secs)
            .ignore();

        let (count_usize,): (usize,) = pipe.query_async(&mut conn).await?;
        let count = u32::try_from(count_usize).unwrap_or(u32::MAX);
        let allowed = count <= limit;
        let retry_after_seconds =
            if allowed { 0 } else { u32::try_from(window_secs).unwrap_or(u32::MAX) };
        Ok(RateLimitDecision { allowed, count, retry_after_seconds })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn decision_fields_round_trip() {
        let d = RateLimitDecision { allowed: false, count: 7, retry_after_seconds: 60 };
        assert!(!d.allowed);
        assert_eq!(d.count, 7);
        assert_eq!(d.retry_after_seconds, 60);
    }
}
