//! Postgres-backed [`HealthRepo`].

use async_trait::async_trait;
use my_fam_tree_domain::{HealthRepo, HealthRepoError};
use sqlx::PgPool;

#[derive(Clone, Debug)]
pub struct PgHealthRepo {
    pool: PgPool,
}

impl PgHealthRepo {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HealthRepo for PgHealthRepo {
    async fn ping(&self) -> Result<(), HealthRepoError> {
        // Touches a migrated core table: proves connectivity AND that the
        // schema is in place. `LIMIT 1` keeps it sub-ms regardless of size;
        // an empty `users` table still succeeds (0 rows, no error).
        // Runtime-checked query (no `.sqlx` entry needed).
        sqlx::query("SELECT 1 FROM users LIMIT 1")
            .execute(&self.pool)
            .await
            .map_err(|e| HealthRepoError::Db(e.to_string()))?;
        Ok(())
    }
}
