//! Statements for the deterministic dev/test seed (`crates/seeder`).
//!
//! Behind the `seed` feature so the api and worker never compile them.
//! Every statement is an idempotent `INSERT … ON CONFLICT … DO UPDATE` or a
//! family-scoped "delete everything not in the seed" reset; the seeder owns
//! the data, this module owns the SQL (arch-tests: SQL lives in persistence).

mod tree;

pub use tree::{ContactRow, ParentLinkRow, PartnershipRow, PersonRow};
use uuid::Uuid;

use crate::{Database, PgMagicLinkRepo};

/// A seed statement failed.
#[derive(Debug, thiserror::Error)]
#[error("seed statement `{statement}` failed")]
pub struct SeedError {
    pub statement: &'static str,
    #[source]
    source: Box<dyn std::error::Error + Send + Sync>,
}

impl SeedError {
    fn new(statement: &'static str) -> impl FnOnce(sqlx::Error) -> Self {
        move |e| Self { statement, source: Box::new(e) }
    }
}

/// Seed statements on the configured pool (same limits as the app's pool).
#[derive(Debug, Clone, Copy)]
pub struct SeedDb<'a> {
    db: &'a Database,
}

impl<'a> SeedDb<'a> {
    #[must_use]
    pub const fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Magic-link repo on the same pool, for minting the printed login URLs.
    #[must_use]
    pub fn magic_links(&self) -> PgMagicLinkRepo {
        PgMagicLinkRepo::new(self.db.pool().clone())
    }

    /// Delete a user holding `email` under an id other than `keep_id` (a
    /// "squatter" that signed up through the app). Dependents cascade.
    ///
    /// # Errors
    /// On any Postgres error.
    pub async fn delete_email_squatter(&self, email: &str, keep_id: Uuid) -> Result<(), SeedError> {
        sqlx::query("DELETE FROM users WHERE email = $1 AND id <> $2")
            .bind(email)
            .bind(keep_id)
            .execute(self.db.pool())
            .await
            .map_err(SeedError::new("delete_email_squatter"))?;
        Ok(())
    }

    /// Upsert a verified user (locale `en`).
    ///
    /// # Errors
    /// On any Postgres error.
    pub async fn upsert_user(
        &self,
        id: Uuid,
        email: &str,
        display_name: &str,
    ) -> Result<(), SeedError> {
        sqlx::query(
            "INSERT INTO users (id, email, display_name, locale, email_verified_at) \
             VALUES ($1, $2, $3, 'en', now()) \
             ON CONFLICT (id) DO UPDATE SET \
                 email = EXCLUDED.email, \
                 display_name = EXCLUDED.display_name, \
                 email_verified_at = COALESCE(users.email_verified_at, EXCLUDED.email_verified_at)",
        )
        .bind(id)
        .bind(email)
        .bind(display_name)
        .execute(self.db.pool())
        .await
        .map_err(SeedError::new("upsert_user"))?;
        Ok(())
    }

    /// Upsert a family.
    ///
    /// # Errors
    /// On any Postgres error.
    pub async fn upsert_family(
        &self,
        id: Uuid,
        name: &str,
        created_by: Uuid,
    ) -> Result<(), SeedError> {
        sqlx::query(
            "INSERT INTO families (id, name, created_by) VALUES ($1, $2, $3) \
             ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name",
        )
        .bind(id)
        .bind(name)
        .bind(created_by)
        .execute(self.db.pool())
        .await
        .map_err(SeedError::new("upsert_family"))?;
        Ok(())
    }

    /// Upsert a membership; `role` is a `family_role` enum label.
    ///
    /// # Errors
    /// On any Postgres error.
    pub async fn upsert_membership(
        &self,
        family_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> Result<(), SeedError> {
        sqlx::query(
            "INSERT INTO family_memberships (family_id, user_id, role) \
             VALUES ($1, $2, ($3::text)::family_role) \
             ON CONFLICT (family_id, user_id) DO UPDATE SET role = EXCLUDED.role",
        )
        .bind(family_id)
        .bind(user_id)
        .bind(role)
        .execute(self.db.pool())
        .await
        .map_err(SeedError::new("upsert_membership"))?;
        Ok(())
    }
}
