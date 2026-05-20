//! Integration test for the sliding-window rate limiter.
//!
//! Requires `REDIS_URL` pointing at a running Redis (compose service
//! `redis`). Silently returns when unset so the test is a no-op locally
//! without infrastructure.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use my_family_cache::{RateLimiter, RedisPool, RedisRateLimiter};

#[tokio::test]
async fn allows_up_to_limit_then_blocks() {
    let Ok(url) = std::env::var("REDIS_URL") else {
        // No REDIS_URL — skip silently. CI runs this with redis as a service.
        return;
    };
    let prefix = format!("rl-test-{}:", uuid::Uuid::new_v4());
    let pool = RedisPool::build(&url, 4, prefix).expect("build pool");
    let limiter = RedisRateLimiter::new(pool);
    let key = "user:1";

    for i in 1..=3 {
        let d = limiter.check(key, 3, Duration::from_mins(1)).await.unwrap();
        assert!(d.allowed, "request {i} should be allowed");
    }

    let d = limiter.check(key, 3, Duration::from_mins(1)).await.unwrap();
    assert!(!d.allowed, "fourth request should be blocked");
    assert!(d.retry_after_seconds > 0);
}
