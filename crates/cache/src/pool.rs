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

    pub fn prefix(&self) -> &str {
        &self.key_prefix
    }

    pub const fn inner(&self) -> &Pool {
        &self.inner
    }

    /// Acquire a connection and issue a `PING` against it.
    pub async fn ping(&self) -> Result<(), CacheError> {
        let mut conn = self.inner.get().await?;
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(())
    }
}
