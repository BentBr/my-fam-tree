//! Postgres-backed [`PartnershipRepo`] implementation.
//!
//! Pair canonicalization (`partner_a_id < partner_b_id`) is enforced by the
//! `CHECK` constraint and re-enforced here so the rows we hand back also
//! satisfy the contract. Duplicate currently-open partnerships are surfaced
//! as `PartnershipRepoError::Duplicate` via the `partnerships_unique_open`
//! partial unique index.

use async_trait::async_trait;
use my_fam_tree_domain::{
    FamilyId, Partnership, PartnershipDraft, PartnershipEndReason, PartnershipKind,
    PartnershipRepo, PartnershipRepoError, PersonId,
};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct PgPartnershipRepo {
    pool: PgPool,
}

impl PgPartnershipRepo {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn kind_from(s: &str) -> PartnershipKind {
    match s {
        "marriage" => PartnershipKind::Marriage,
        "civil_union" => PartnershipKind::CivilUnion,
        _ => PartnershipKind::Partnership,
    }
}

fn end_from(s: Option<&str>) -> Option<PartnershipEndReason> {
    match s {
        Some("divorce") => Some(PartnershipEndReason::Divorce),
        Some("separation") => Some(PartnershipEndReason::Separation),
        Some("death") => Some(PartnershipEndReason::Death),
        _ => None,
    }
}

#[async_trait]
impl PartnershipRepo for PgPartnershipRepo {
    async fn create(
        &self,
        family_id: FamilyId,
        a: PersonId,
        b: PersonId,
        d: PartnershipDraft,
    ) -> Result<Partnership, PartnershipRepoError> {
        if a == b {
            return Err(PartnershipRepoError::Duplicate);
        }
        let (lo, hi) = if a.into_uuid() < b.into_uuid() { (a, b) } else { (b, a) };
        let end_str: Option<&str> = d.end_reason.map(PartnershipEndReason::as_db);
        let res = sqlx::query!(
            r#"INSERT INTO partnerships
                 (family_id, partner_a_id, partner_b_id, kind, started_on, ended_on, end_reason, note)
               VALUES ($1, $2, $3, ($4::text)::partnership_kind, $5, $6,
                       ($7::text)::partnership_end_reason, $8)
               RETURNING id, started_on, ended_on, end_reason::text AS end_reason, note"#,
            family_id.into_uuid(),
            lo.into_uuid(),
            hi.into_uuid(),
            d.kind.as_db(),
            d.started_on,
            d.ended_on,
            end_str,
            d.note,
        )
        .fetch_one(&self.pool)
        .await;
        match res {
            Ok(r) => Ok(Partnership {
                id: r.id,
                family_id,
                partner_a_id: lo,
                partner_b_id: hi,
                kind: d.kind,
                started_on: r.started_on,
                ended_on: r.ended_on,
                end_reason: end_from(r.end_reason.as_deref()),
                note: r.note,
            }),
            Err(sqlx::Error::Database(db))
                if db.constraint() == Some("partnerships_unique_open") =>
            {
                Err(PartnershipRepoError::Duplicate)
            }
            Err(e) => Err(PartnershipRepoError::Db(e.to_string())),
        }
    }

    async fn list_for_family(
        &self,
        family_id: FamilyId,
    ) -> Result<Vec<Partnership>, PartnershipRepoError> {
        let rows = sqlx::query!(
            r#"SELECT id, family_id, partner_a_id, partner_b_id, kind::text AS "kind!",
                      started_on, ended_on, end_reason::text AS end_reason, note
                 FROM partnerships WHERE family_id = $1
                 ORDER BY started_on NULLS LAST, id"#,
            family_id.into_uuid()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PartnershipRepoError::Db(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|r| Partnership {
                id: r.id,
                family_id: FamilyId::from_uuid(r.family_id),
                partner_a_id: PersonId::from_uuid(r.partner_a_id),
                partner_b_id: PersonId::from_uuid(r.partner_b_id),
                kind: kind_from(&r.kind),
                started_on: r.started_on,
                ended_on: r.ended_on,
                end_reason: end_from(r.end_reason.as_deref()),
                note: r.note,
            })
            .collect())
    }

    async fn update(
        &self,
        family_id: FamilyId,
        id: Uuid,
        d: PartnershipDraft,
    ) -> Result<Partnership, PartnershipRepoError> {
        let end_str: Option<&str> = d.end_reason.map(PartnershipEndReason::as_db);
        let res = sqlx::query!(
            r#"UPDATE partnerships
                  SET kind = ($3::text)::partnership_kind,
                      started_on = $4,
                      ended_on = $5,
                      end_reason = ($6::text)::partnership_end_reason,
                      note = $7
                WHERE family_id = $1 AND id = $2
                RETURNING id, family_id, partner_a_id, partner_b_id, kind::text AS "kind!",
                          started_on, ended_on, end_reason::text AS end_reason, note"#,
            family_id.into_uuid(),
            id,
            d.kind.as_db(),
            d.started_on,
            d.ended_on,
            end_str,
            d.note,
        )
        .fetch_optional(&self.pool)
        .await;
        match res {
            Ok(Some(r)) => Ok(Partnership {
                id: r.id,
                family_id: FamilyId::from_uuid(r.family_id),
                partner_a_id: PersonId::from_uuid(r.partner_a_id),
                partner_b_id: PersonId::from_uuid(r.partner_b_id),
                kind: kind_from(&r.kind),
                started_on: r.started_on,
                ended_on: r.ended_on,
                end_reason: end_from(r.end_reason.as_deref()),
                note: r.note,
            }),
            Ok(None) => Err(PartnershipRepoError::NotFound),
            Err(sqlx::Error::Database(db))
                if db.constraint() == Some("partnerships_unique_open") =>
            {
                Err(PartnershipRepoError::Duplicate)
            }
            Err(e) => Err(PartnershipRepoError::Db(e.to_string())),
        }
    }

    async fn delete(&self, family_id: FamilyId, id: Uuid) -> Result<(), PartnershipRepoError> {
        let res = sqlx::query!(
            "DELETE FROM partnerships WHERE family_id = $1 AND id = $2",
            family_id.into_uuid(),
            id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| PartnershipRepoError::Db(e.to_string()))?;
        if res.rows_affected() == 0 {
            return Err(PartnershipRepoError::NotFound);
        }
        Ok(())
    }
}
