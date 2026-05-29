//! Postgres-backed [`PersonContactRepo`] implementation.
//!
//! The two `contact_kind` / `contact_visibility` enums are bound as
//! `text` and cast to the enum on the SQL side (the same pattern used
//! elsewhere in this crate — see `family_memberships` / `partnerships`).

use async_trait::async_trait;
use my_fam_tree_domain::{
    Contact, ContactDraft, ContactKind, ContactRepoError, ContactVisibility, PersonContactRepo,
    PersonId,
};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct PgPersonContactRepo {
    pool: PgPool,
}

impl PgPersonContactRepo {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn kind_from(s: &str) -> ContactKind {
    match s {
        "email" => ContactKind::Email,
        "phone" => ContactKind::Phone,
        "address" => ContactKind::Address,
        "url" => ContactKind::Url,
        _ => ContactKind::Other,
    }
}

fn vis_from(s: &str) -> ContactVisibility {
    if s == "admins_only" { ContactVisibility::AdminsOnly } else { ContactVisibility::Family }
}

#[async_trait]
impl PersonContactRepo for PgPersonContactRepo {
    async fn list_for_person(&self, person_id: PersonId) -> Result<Vec<Contact>, ContactRepoError> {
        let rows = sqlx::query!(
            r#"SELECT id, person_id,
                      kind::text AS "kind!",
                      label,
                      value AS "value!: serde_json::Value",
                      visibility::text AS "vis!"
                 FROM person_contacts WHERE person_id = $1 ORDER BY kind, label"#,
            person_id.into_uuid()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ContactRepoError::Db(e.to_string()))?;
        Ok(rows
            .into_iter()
            .map(|r| Contact {
                id: r.id,
                person_id: PersonId::from_uuid(r.person_id),
                kind: kind_from(&r.kind),
                label: r.label,
                value: r.value,
                visibility: vis_from(&r.vis),
            })
            .collect())
    }

    async fn create(
        &self,
        person_id: PersonId,
        d: ContactDraft,
    ) -> Result<Contact, ContactRepoError> {
        let kind_db = d.kind.as_db();
        let vis_db = d.visibility.as_db();
        let r = sqlx::query!(
            r#"INSERT INTO person_contacts (person_id, kind, label, value, visibility)
               VALUES ($1, ($2::text)::contact_kind, $3, $4, ($5::text)::contact_visibility)
               RETURNING id, person_id,
                         kind::text AS "kind!",
                         label,
                         value AS "value!: serde_json::Value",
                         visibility::text AS "vis!""#,
            person_id.into_uuid(),
            kind_db,
            d.label,
            d.value,
            vis_db,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ContactRepoError::Db(e.to_string()))?;
        Ok(Contact {
            id: r.id,
            person_id: PersonId::from_uuid(r.person_id),
            kind: kind_from(&r.kind),
            label: r.label,
            value: r.value,
            visibility: vis_from(&r.vis),
        })
    }

    async fn update(&self, id: Uuid, d: ContactDraft) -> Result<Contact, ContactRepoError> {
        let kind_db = d.kind.as_db();
        let vis_db = d.visibility.as_db();
        let r = sqlx::query!(
            r#"UPDATE person_contacts SET kind = ($2::text)::contact_kind, label = $3, value = $4,
                                          visibility = ($5::text)::contact_visibility
                WHERE id = $1
                RETURNING id, person_id,
                          kind::text AS "kind!",
                          label,
                          value AS "value!: serde_json::Value",
                          visibility::text AS "vis!""#,
            id,
            kind_db,
            d.label,
            d.value,
            vis_db,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContactRepoError::Db(e.to_string()))?
        .ok_or(ContactRepoError::NotFound)?;
        Ok(Contact {
            id: r.id,
            person_id: PersonId::from_uuid(r.person_id),
            kind: kind_from(&r.kind),
            label: r.label,
            value: r.value,
            visibility: vis_from(&r.vis),
        })
    }

    async fn delete(&self, id: Uuid) -> Result<(), ContactRepoError> {
        let res = sqlx::query!("DELETE FROM person_contacts WHERE id = $1", id)
            .execute(&self.pool)
            .await
            .map_err(|e| ContactRepoError::Db(e.to_string()))?;
        if res.rows_affected() == 0 {
            return Err(ContactRepoError::NotFound);
        }
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Contact>, ContactRepoError> {
        let row = sqlx::query!(
            r#"SELECT id, person_id,
                      kind::text AS "kind!",
                      label,
                      value AS "value!: serde_json::Value",
                      visibility::text AS "vis!"
                 FROM person_contacts WHERE id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ContactRepoError::Db(e.to_string()))?;
        Ok(row.map(|r| Contact {
            id: r.id,
            person_id: PersonId::from_uuid(r.person_id),
            kind: kind_from(&r.kind),
            label: r.label,
            value: r.value,
            visibility: vis_from(&r.vis),
        }))
    }
}
