//! Postgres-backed [`PersonFavouriteRepo`].
//!
//! Three pieces of SQL:
//! - `set` — `INSERT ... ON CONFLICT (user_id, person_id) DO NOTHING`. The
//!   composite primary key from `migrations/0008_person_favourites.sql`
//!   makes this idempotent end-to-end.
//! - `unset` — plain `DELETE`. We do NOT error when no row matched —
//!   double-unset must also succeed so the FE can spam-toggle without
//!   the BE returning 404 on a race.
//! - `list_for_user` — joins `person_favourites` with `persons` on the
//!   family scope so cross-family favourites can't leak through (a user
//!   may belong to multiple families and the favourite set is global by
//!   `(user, person)` but the projection must stay family-scoped).

use std::collections::HashSet;

use async_trait::async_trait;
use my_fam_tree_domain::{
    FamilyId, PersonFavouriteRepo, PersonFavouriteRepoError, PersonId, UserId,
};
use sqlx::PgPool;

#[derive(Clone, Debug)]
pub struct PgPersonFavouriteRepo {
    pool: PgPool,
}

impl PgPersonFavouriteRepo {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PersonFavouriteRepo for PgPersonFavouriteRepo {
    async fn set(
        &self,
        user_id: UserId,
        person_id: PersonId,
    ) -> Result<(), PersonFavouriteRepoError> {
        sqlx::query!(
            r#"INSERT INTO person_favourites (user_id, person_id)
               VALUES ($1, $2)
               ON CONFLICT (user_id, person_id) DO NOTHING"#,
            user_id.into_uuid(),
            person_id.into_uuid(),
        )
        .execute(&self.pool)
        .await
        .map_err(|e| PersonFavouriteRepoError::Db(e.to_string()))?;
        Ok(())
    }

    async fn unset(
        &self,
        user_id: UserId,
        person_id: PersonId,
    ) -> Result<(), PersonFavouriteRepoError> {
        sqlx::query!(
            r#"DELETE FROM person_favourites WHERE user_id = $1 AND person_id = $2"#,
            user_id.into_uuid(),
            person_id.into_uuid(),
        )
        .execute(&self.pool)
        .await
        .map_err(|e| PersonFavouriteRepoError::Db(e.to_string()))?;
        Ok(())
    }

    async fn list_for_user(
        &self,
        user_id: UserId,
        family_id: FamilyId,
    ) -> Result<HashSet<PersonId>, PersonFavouriteRepoError> {
        let rows = sqlx::query!(
            r#"SELECT pf.person_id
                 FROM person_favourites pf
                 JOIN persons p ON p.id = pf.person_id
                WHERE pf.user_id = $1 AND p.family_id = $2"#,
            user_id.into_uuid(),
            family_id.into_uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PersonFavouriteRepoError::Db(e.to_string()))?;
        Ok(rows.into_iter().map(|r| PersonId::from_uuid(r.person_id)).collect())
    }

    async fn is_favourite_for_user(
        &self,
        user_id: UserId,
        person_id: PersonId,
    ) -> Result<bool, PersonFavouriteRepoError> {
        let row = sqlx::query!(
            r#"SELECT 1 AS "x!" FROM person_favourites
                WHERE user_id = $1 AND person_id = $2"#,
            user_id.into_uuid(),
            person_id.into_uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PersonFavouriteRepoError::Db(e.to_string()))?;
        Ok(row.is_some())
    }
}
