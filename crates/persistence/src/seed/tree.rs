//! Seed statements for persons, relationships and contacts.

use serde_json::Value;
use time::Date;
use uuid::Uuid;

use super::{SeedDb, SeedError};

/// One `persons` row.
#[derive(Debug, Clone, Copy)]
pub struct PersonRow<'a> {
    pub id: Uuid,
    pub given_name: &'a str,
    pub family_name: &'a str,
    pub name_at_birth: &'a str,
    pub nickname: &'a str,
    /// `gender` column value.
    pub gender: &'a str,
    pub birth_date: Date,
    pub birth_place: &'a str,
    pub death_date: Option<Date>,
    pub notes: &'a str,
    pub linked_user_id: Option<Uuid>,
}

/// One `parent_links` row; `kind` is a `parent_link_kind` enum label.
#[derive(Debug, Clone, Copy)]
pub struct ParentLinkRow<'a> {
    pub child: Uuid,
    pub parent: Uuid,
    pub kind: &'a str,
}

/// One `partnerships` row; `kind` / `end_reason` are enum labels.
#[derive(Debug, Clone, Copy)]
pub struct PartnershipRow<'a> {
    pub id: Uuid,
    pub partner_a: Uuid,
    pub partner_b: Uuid,
    pub kind: &'a str,
    pub started_on: Option<Date>,
    pub ended_on: Option<Date>,
    pub end_reason: Option<&'a str>,
    pub note: &'a str,
}

/// One `person_contacts` row; `kind` / `visibility` are enum labels.
#[derive(Debug, Clone, Copy)]
pub struct ContactRow<'a> {
    pub id: Uuid,
    pub person_id: Uuid,
    pub kind: &'a str,
    pub label: &'a str,
    pub value: &'a Value,
    pub visibility: &'a str,
}

impl SeedDb<'_> {
    /// Delete every person in `family_id` whose id is not in `keep`
    /// (dependent links, partnerships and contacts cascade).
    ///
    /// # Errors
    /// On any Postgres error.
    pub async fn delete_persons_except(
        &self,
        family_id: Uuid,
        keep: &[Uuid],
    ) -> Result<(), SeedError> {
        sqlx::query("DELETE FROM persons WHERE family_id = $1 AND id <> ALL($2)")
            .bind(family_id)
            .bind(keep)
            .execute(self.db.pool())
            .await
            .map_err(SeedError::new("delete_persons_except"))?;
        Ok(())
    }

    /// Upsert one person into `family_id`.
    ///
    /// # Errors
    /// On any Postgres error.
    pub async fn upsert_person(&self, family_id: Uuid, p: &PersonRow<'_>) -> Result<(), SeedError> {
        sqlx::query(
            "INSERT INTO persons \
                 (id, family_id, given_name, family_name, name_at_birth, nickname, gender, \
                  birth_date, birth_place, death_date, notes, \
                  linked_user_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
             ON CONFLICT (id) DO UPDATE SET \
                 family_id = EXCLUDED.family_id, \
                 given_name = EXCLUDED.given_name, \
                 family_name = EXCLUDED.family_name, \
                 name_at_birth = EXCLUDED.name_at_birth, \
                 nickname = EXCLUDED.nickname, \
                 gender = EXCLUDED.gender, \
                 birth_date = EXCLUDED.birth_date, \
                 birth_place = EXCLUDED.birth_place, \
                 death_date = EXCLUDED.death_date, \
                 notes = EXCLUDED.notes, \
                 linked_user_id = EXCLUDED.linked_user_id",
        )
        .bind(p.id)
        .bind(family_id)
        .bind(p.given_name)
        .bind(p.family_name)
        .bind(p.name_at_birth)
        .bind(p.nickname)
        .bind(p.gender)
        .bind(p.birth_date)
        .bind(p.birth_place)
        .bind(p.death_date)
        .bind(p.notes)
        .bind(p.linked_user_id)
        .execute(self.db.pool())
        .await
        .map_err(SeedError::new("upsert_person"))?;
        Ok(())
    }

    /// Upsert one parent link.
    ///
    /// # Errors
    /// On any Postgres error.
    pub async fn upsert_parent_link(&self, r: &ParentLinkRow<'_>) -> Result<(), SeedError> {
        sqlx::query(
            "INSERT INTO parent_links (child_id, parent_id, kind, note) \
             VALUES ($1, $2, ($3::text)::parent_link_kind, '') \
             ON CONFLICT (child_id, parent_id) DO UPDATE SET \
                 kind = EXCLUDED.kind, \
                 note = EXCLUDED.note",
        )
        .bind(r.child)
        .bind(r.parent)
        .bind(r.kind)
        .execute(self.db.pool())
        .await
        .map_err(SeedError::new("upsert_parent_link"))?;
        Ok(())
    }

    /// Delete every partnership in `family_id` whose id is not in `keep`.
    ///
    /// # Errors
    /// On any Postgres error.
    pub async fn delete_partnerships_except(
        &self,
        family_id: Uuid,
        keep: &[Uuid],
    ) -> Result<(), SeedError> {
        sqlx::query("DELETE FROM partnerships WHERE family_id = $1 AND id <> ALL($2)")
            .bind(family_id)
            .bind(keep)
            .execute(self.db.pool())
            .await
            .map_err(SeedError::new("delete_partnerships_except"))?;
        Ok(())
    }

    /// Upsert one partnership into `family_id`.
    ///
    /// # Errors
    /// On any Postgres error.
    pub async fn upsert_partnership(
        &self,
        family_id: Uuid,
        p: &PartnershipRow<'_>,
    ) -> Result<(), SeedError> {
        sqlx::query(
            "INSERT INTO partnerships \
                 (id, family_id, partner_a_id, partner_b_id, kind, started_on, ended_on, \
                  end_reason, note) \
             VALUES ($1, $2, $3, $4, ($5::text)::partnership_kind, $6, $7, \
                     $8::text::partnership_end_reason, $9) \
             ON CONFLICT (id) DO UPDATE SET \
                 family_id = EXCLUDED.family_id, \
                 partner_a_id = EXCLUDED.partner_a_id, \
                 partner_b_id = EXCLUDED.partner_b_id, \
                 kind = EXCLUDED.kind, \
                 started_on = EXCLUDED.started_on, \
                 ended_on = EXCLUDED.ended_on, \
                 end_reason = EXCLUDED.end_reason, \
                 note = EXCLUDED.note",
        )
        .bind(p.id)
        .bind(family_id)
        .bind(p.partner_a)
        .bind(p.partner_b)
        .bind(p.kind)
        .bind(p.started_on)
        .bind(p.ended_on)
        .bind(p.end_reason)
        .bind(p.note)
        .execute(self.db.pool())
        .await
        .map_err(SeedError::new("upsert_partnership"))?;
        Ok(())
    }

    /// Upsert one contact.
    ///
    /// # Errors
    /// On any Postgres error.
    pub async fn upsert_contact(&self, c: &ContactRow<'_>) -> Result<(), SeedError> {
        sqlx::query(
            "INSERT INTO person_contacts (id, person_id, kind, label, value, visibility) \
             VALUES ($1, $2, ($3::text)::contact_kind, $4, $5, ($6::text)::contact_visibility) \
             ON CONFLICT (id) DO UPDATE SET \
                 person_id = EXCLUDED.person_id, \
                 kind = EXCLUDED.kind, \
                 label = EXCLUDED.label, \
                 value = EXCLUDED.value, \
                 visibility = EXCLUDED.visibility",
        )
        .bind(c.id)
        .bind(c.person_id)
        .bind(c.kind)
        .bind(c.label)
        .bind(c.value)
        .bind(c.visibility)
        .execute(self.db.pool())
        .await
        .map_err(SeedError::new("upsert_contact"))?;
        Ok(())
    }
}
