//! Postgres-backed [`ParentLinkRepo`] implementation.
//!
//! The `insert` method wraps the cycle check + insert in a single
//! `SERIALIZABLE` transaction so concurrent writers cannot bypass the
//! in-memory cycle check that the routes layer ran on a stale snapshot.
//! On a serialization conflict the caller may retry; we surface a generic
//! `Db` error so the route maps it to `Internal` (rare in practice).

use async_trait::async_trait;
use my_family_domain::{
    FamilyId, ParentKind, ParentLink, ParentLinkRepo, ParentLinkRepoError, PersonId,
    would_create_cycle,
};
use sqlx::{PgPool, Postgres, Transaction};

#[derive(Clone, Debug)]
pub struct PgParentLinkRepo {
    pool: PgPool,
}

impl PgParentLinkRepo {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn kind_from(s: &str) -> ParentKind {
    match s {
        "biological" => ParentKind::Biological,
        "legal" => ParentKind::Legal,
        "adoptive" => ParentKind::Adoptive,
        "step" => ParentKind::Step,
        _ => ParentKind::Social,
    }
}

async fn fetch_edges_for_check(
    tx: &mut Transaction<'_, Postgres>,
    family_id: FamilyId,
) -> Result<Vec<(PersonId, PersonId)>, ParentLinkRepoError> {
    let rows = sqlx::query!(
        r#"SELECT pl.child_id, pl.parent_id FROM parent_links pl
           JOIN persons p ON p.id = pl.child_id WHERE p.family_id = $1"#,
        family_id.into_uuid()
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| ParentLinkRepoError::Db(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|r| (PersonId::from_uuid(r.child_id), PersonId::from_uuid(r.parent_id)))
        .collect())
}

#[async_trait]
impl ParentLinkRepo for PgParentLinkRepo {
    async fn all_edges_in_family(
        &self,
        family_id: FamilyId,
    ) -> Result<Vec<(PersonId, PersonId)>, ParentLinkRepoError> {
        let rows = sqlx::query!(
            r#"SELECT pl.child_id, pl.parent_id FROM parent_links pl
               JOIN persons p ON p.id = pl.child_id WHERE p.family_id = $1"#,
            family_id.into_uuid()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ParentLinkRepoError::Db(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|r| (PersonId::from_uuid(r.child_id), PersonId::from_uuid(r.parent_id)))
            .collect())
    }

    async fn list_for_family(
        &self,
        family_id: FamilyId,
    ) -> Result<Vec<ParentLink>, ParentLinkRepoError> {
        let rows = sqlx::query!(
            r#"SELECT pl.child_id, pl.parent_id, pl.kind::text AS "kind!", pl.note
                 FROM parent_links pl JOIN persons p ON p.id = pl.child_id
                WHERE p.family_id = $1"#,
            family_id.into_uuid()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ParentLinkRepoError::Db(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|r| ParentLink {
                child_id: PersonId::from_uuid(r.child_id),
                parent_id: PersonId::from_uuid(r.parent_id),
                kind: kind_from(&r.kind),
                note: r.note,
            })
            .collect())
    }

    async fn insert(
        &self,
        family_id: FamilyId,
        child_id: PersonId,
        parent_id: PersonId,
        kind: ParentKind,
        note: &str,
    ) -> Result<(), ParentLinkRepoError> {
        if child_id == parent_id {
            return Err(ParentLinkRepoError::SelfParent);
        }

        // SERIALIZABLE closes the TOCTOU window between the cycle-check read
        // and the insert. If another concurrent writer commits a conflicting
        // edge first, Postgres aborts our transaction with serialization
        // failure; we surface that as a generic `Db` error and let the caller
        // retry.
        let mut tx = self.pool.begin().await.map_err(|e| ParentLinkRepoError::Db(e.to_string()))?;
        sqlx::query!("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
            .execute(&mut *tx)
            .await
            .map_err(|e| ParentLinkRepoError::Db(e.to_string()))?;

        let edges = fetch_edges_for_check(&mut tx, family_id).await?;
        if would_create_cycle(&edges, child_id, parent_id) {
            return Err(ParentLinkRepoError::Cycle);
        }

        sqlx::query!(
            "INSERT INTO parent_links (child_id, parent_id, kind, note)
             VALUES ($1, $2, ($3::text)::parent_link_kind, $4)
             ON CONFLICT (child_id, parent_id) DO UPDATE
                 SET kind = EXCLUDED.kind, note = EXCLUDED.note",
            child_id.into_uuid(),
            parent_id.into_uuid(),
            kind.as_db(),
            note
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ParentLinkRepoError::Db(e.to_string()))?;

        tx.commit().await.map_err(|e| ParentLinkRepoError::Db(e.to_string()))?;
        Ok(())
    }

    async fn delete(
        &self,
        _family_id: FamilyId,
        child_id: PersonId,
        parent_id: PersonId,
    ) -> Result<(), ParentLinkRepoError> {
        let res = sqlx::query!(
            "DELETE FROM parent_links WHERE child_id = $1 AND parent_id = $2",
            child_id.into_uuid(),
            parent_id.into_uuid()
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ParentLinkRepoError::Db(e.to_string()))?;
        if res.rows_affected() == 0 {
            return Err(ParentLinkRepoError::NotFound);
        }
        Ok(())
    }
}
