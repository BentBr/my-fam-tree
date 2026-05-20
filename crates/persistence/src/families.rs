//! Postgres-backed [`FamilyRepo`] implementation.

use async_trait::async_trait;
use my_family_domain::{Family, FamilyId, FamilyRepo, FamilyRepoError, UserId};
use sqlx::PgPool;

#[derive(Clone, Debug)]
pub struct PgFamilyRepo {
    pool: PgPool,
}

impl PgFamilyRepo {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FamilyRepo for PgFamilyRepo {
    async fn create(&self, name: &str, created_by: UserId) -> Result<Family, FamilyRepoError> {
        let row = sqlx::query!(
            r#"INSERT INTO families (name, created_by) VALUES ($1, $2)
               RETURNING id, name, created_by, created_at"#,
            name,
            created_by.into_uuid()
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| FamilyRepoError::Db(e.to_string()))?;
        Ok(Family {
            id: FamilyId::from_uuid(row.id),
            name: row.name,
            created_by: UserId::from_uuid(row.created_by),
            created_at: row.created_at,
        })
    }

    async fn find_by_id(&self, id: FamilyId) -> Result<Option<Family>, FamilyRepoError> {
        let row = sqlx::query!(
            r#"SELECT id, name, created_by, created_at FROM families WHERE id = $1"#,
            id.into_uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| FamilyRepoError::Db(e.to_string()))?;
        Ok(row.map(|r| Family {
            id: FamilyId::from_uuid(r.id),
            name: r.name,
            created_by: UserId::from_uuid(r.created_by),
            created_at: r.created_at,
        }))
    }

    async fn rename(&self, id: FamilyId, name: &str) -> Result<(), FamilyRepoError> {
        let res =
            sqlx::query!(r#"UPDATE families SET name = $2 WHERE id = $1"#, id.into_uuid(), name)
                .execute(&self.pool)
                .await
                .map_err(|e| FamilyRepoError::Db(e.to_string()))?;
        if res.rows_affected() == 0 {
            return Err(FamilyRepoError::NotFound);
        }
        Ok(())
    }

    async fn delete(&self, id: FamilyId) -> Result<(), FamilyRepoError> {
        let res = sqlx::query!(r#"DELETE FROM families WHERE id = $1"#, id.into_uuid())
            .execute(&self.pool)
            .await
            .map_err(|e| FamilyRepoError::Db(e.to_string()))?;
        if res.rows_affected() == 0 {
            return Err(FamilyRepoError::NotFound);
        }
        Ok(())
    }
}
