//! Typed error for the cache layer.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("redis pool: {0}")]
    Pool(#[from] deadpool_redis::PoolError),
    #[error("redis: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("config: {0}")]
    Config(String),
}
