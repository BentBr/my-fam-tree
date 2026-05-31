//! Redis connection pool wrapper.

use deadpool_redis::{Config, Pool, Runtime};

use crate::error::CacheError;

#[derive(Clone, Debug)]
pub struct RedisPool {
    inner: Pool,
    key_prefix: String,
}

impl RedisPool {
    /// Build a new pool from a Redis URL with the given max size and key prefix.
    ///
    /// # Errors
    /// Returns [`CacheError::Config`] if the URL is malformed or the
    /// `deadpool-redis` builder rejects the configuration.
    pub fn build(
        url: &str,
        max_size: usize,
        key_prefix: impl Into<String>,
    ) -> Result<Self, CacheError> {
        let cfg = Config::from_url(url);
        let builder = cfg
            .builder()
            .map_err(|e| CacheError::Config(e.to_string()))?
            .max_size(max_size)
            .runtime(Runtime::Tokio1);
        let inner = builder.build().map_err(|e| CacheError::Config(e.to_string()))?;
        Ok(Self { inner, key_prefix: key_prefix.into() })
    }

    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.key_prefix
    }

    #[must_use]
    pub const fn inner(&self) -> &Pool {
        &self.inner
    }

    /// Acquire a connection and issue a `PING` against it.
    ///
    /// # Errors
    /// Returns [`CacheError::Pool`] if no connection can be acquired, or
    /// [`CacheError::Redis`] if the server rejects the `PING` command.
    pub async fn ping(&self) -> Result<(), CacheError> {
        let mut conn = self.inner.get().await?;
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(())
    }

    /// Check whether a key under our prefix currently exists.
    ///
    /// `suffix` is appended to the pool's prefix verbatim — callers pass the
    /// trailing part (e.g. `"reminder:leader"`) so this method owns the
    /// prefix-vs-suffix join exactly like the rest of the cache layer.
    ///
    /// # Errors
    /// Returns [`CacheError::Pool`] if no connection can be acquired, or
    /// [`CacheError::Redis`] if the server rejects the `EXISTS` command.
    /// A successful `EXISTS` that returns 0 is `Ok(false)`, not an error —
    /// the absence of a key is a normal state, not a fault.
    pub async fn exists(&self, suffix: &str) -> Result<bool, CacheError> {
        let mut conn = self.inner.get().await?;
        let key = format!("{}{suffix}", self.key_prefix);
        let count: i64 = redis::cmd("EXISTS").arg(&key).query_async(&mut conn).await?;
        Ok(count > 0)
    }
}
