//! Single-leader election via a Redis lease, so only one worker replica ticks.

use std::time::Duration;

use my_fam_tree_cache::RedisPool;
use tokio::time::sleep;

/// A best-effort leader lease.
///
/// `acquire_blocking` waits until it wins the lease key; `refresh` extends it
/// (and signals loss when another instance took it); the lease auto-expires
/// after `ttl` so a crashed leader doesn't deadlock.
#[derive(Debug)]
pub struct Leader {
    pool: RedisPool,
    key: String,
    instance: String,
    ttl: Duration,
}

impl Leader {
    #[must_use]
    pub fn new(pool: RedisPool, ttl: Duration) -> Self {
        let key = format!("{}reminder:leader", pool.prefix());
        let instance = uuid::Uuid::new_v4().to_string();
        Self { pool, key, instance, ttl }
    }

    /// Block (polling every 30s) until this instance holds the lease.
    pub async fn acquire_blocking(&self) {
        loop {
            if self.try_acquire().await {
                return;
            }
            sleep(Duration::from_secs(30)).await;
        }
    }

    /// `SET key instance NX PX ttl` — wins only if the key is unset.
    pub async fn try_acquire(&self) -> bool {
        let Ok(mut conn) = self.pool.inner().get().await else { return false };
        let ttl_ms = u64::try_from(self.ttl.as_millis()).unwrap_or(u64::MAX);
        let res: redis::RedisResult<Option<String>> = redis::cmd("SET")
            .arg(&self.key)
            .arg(&self.instance)
            .arg("NX")
            .arg("PX")
            .arg(ttl_ms)
            .query_async(&mut conn)
            .await;
        matches!(res, Ok(Some(_)))
    }

    /// Extend the lease iff we still own it. Returns false when ownership was
    /// lost (another instance took over) or Redis is unreachable.
    pub async fn refresh(&self) -> bool {
        let Ok(mut conn) = self.pool.inner().get().await else { return false };
        let ttl_ms = u64::try_from(self.ttl.as_millis()).unwrap_or(u64::MAX);
        let script = r"
            if redis.call('GET', KEYS[1]) == ARGV[1] then
                return redis.call('PEXPIRE', KEYS[1], ARGV[2])
            else
                return 0
            end";
        let res: redis::RedisResult<i64> = redis::Script::new(script)
            .key(&self.key)
            .arg(&self.instance)
            .arg(ttl_ms)
            .invoke_async(&mut conn)
            .await;
        matches!(res, Ok(1))
    }
}
