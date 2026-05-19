//! Redis-backed cache, rate limiting, locks, job queue.

pub mod error;
pub mod pool;

pub use error::CacheError;
pub use pool::RedisPool;
