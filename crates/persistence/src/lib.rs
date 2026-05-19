//! SQLx-backed repositories. No business logic here.

pub mod error;
pub mod pool;

pub use error::PersistenceError;
pub use pool::Database;
