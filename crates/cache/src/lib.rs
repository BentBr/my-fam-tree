//! Redis-backed cache, rate limiting, locks, job queue.

pub mod error;
pub mod pool;
pub mod rate_limit;

pub use error::CacheError;
pub use pool::RedisPool;
pub use rate_limit::{RateLimitDecision, RateLimiter, RedisRateLimiter};
