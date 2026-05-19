//! Requires `REDIS_URL` pointing at a running Redis (compose service `redis`).
//! Skipped automatically when `REDIS_URL` is unset.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stderr)]

use my_family_cache::RedisPool;

#[tokio::test]
async fn pings_redis() {
    let Ok(url) = std::env::var("REDIS_URL") else {
        eprintln!("REDIS_URL not set; skipping");
        return;
    };
    let pool = RedisPool::build(&url, 4, "test:").expect("build pool");
    pool.ping().await.expect("ping should succeed");
}
