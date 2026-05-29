//! Postgres-backed [`PersonRepo`] implementation.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use my_family_domain::{
    FamilyId, Person, PersonDraft, PersonId, PersonRepo, PersonRepoError, UserId,
};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct PgPersonRepo {
    pool: PgPool,
}

impl PgPersonRepo {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Mirror of the columns selected by every `persons` query in this file.
/// Centralized so the row → `Person` conversion lives in exactly one place.
#[derive(sqlx::FromRow)]
struct PersonRow {
    id: Uuid,
    family_id: Uuid,
    given_name: String,
    family_name: String,
    name_at_birth: String,
    nickname: String,
    gender: String,
    birth_date: Option<NaiveDate>,
    birth_place: String,
    death_date: Option<NaiveDate>,
    notes: String,
    linked_user_id: Option<Uuid>,
    photo_key: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<PersonRow> for Person {
    fn from(r: PersonRow) -> Self {
        Self {
            id: PersonId::from_uuid(r.id),
            family_id: FamilyId::from_uuid(r.family_id),
            given_name: r.given_name,
            family_name: r.family_name,
            name_at_birth: r.name_at_birth,
            nickname: r.nickname,
            gender: r.gender,
            birth_date: r.birth_date,
            birth_place: r.birth_place,
            death_date: r.death_date,
            notes: r.notes,
            linked_user_id: r.linked_user_id.map(UserId::from_uuid),
            photo_key: r.photo_key,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl PersonRepo for PgPersonRepo {
    async fn create(&self, family_id: FamilyId, d: PersonDraft) -> Result<Person, PersonRepoError> {
        let res = sqlx::query_as!(
            PersonRow,
            r#"INSERT INTO persons
               (family_id, given_name, family_name, name_at_birth, nickname, gender,
                birth_date, birth_place, death_date, notes,
                linked_user_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
               RETURNING id, family_id, given_name, family_name, name_at_birth, nickname,
                         gender, birth_date, birth_place, death_date, notes,
                         linked_user_id, photo_key, created_at, updated_at"#,
            family_id.into_uuid(),
            d.given_name,
            d.family_name,
            d.name_at_birth,
            d.nickname,
            d.gender,
            d.birth_date,
            d.birth_place,
            d.death_date,
            d.notes,
            d.linked_user_id.map(UserId::into_uuid),
        )
        .fetch_one(&self.pool)
        .await;
        match res {
            Ok(r) => Ok(r.into()),
            Err(sqlx::Error::Database(db))
                if db.constraint() == Some("persons_family_id_linked_user_id_key") =>
            {
                Err(PersonRepoError::LinkedUserConflict)
            }
            Err(e) => Err(PersonRepoError::Db(e.to_string())),
        }
    }

    async fn find_in_family(
        &self,
        family_id: FamilyId,
        id: PersonId,
    ) -> Result<Option<Person>, PersonRepoError> {
        let row = sqlx::query_as!(
            PersonRow,
            r#"SELECT id, family_id, given_name, family_name, name_at_birth, nickname,
                      gender, birth_date, birth_place, death_date, notes,
                      linked_user_id, photo_key, created_at, updated_at
                 FROM persons WHERE family_id = $1 AND id = $2"#,
            family_id.into_uuid(),
            id.into_uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PersonRepoError::Db(e.to_string()))?;
        Ok(row.map(Into::into))
    }

    async fn list_for_family(
        &self,
        family_id: FamilyId,
        cursor: Option<PersonId>,
        limit: u32,
    ) -> Result<Vec<Person>, PersonRepoError> {
        let lim = i64::from(limit.clamp(1, 100));
        let rows = match cursor {
            None => {
                sqlx::query_as!(
                    PersonRow,
                    r#"SELECT id, family_id, given_name, family_name, name_at_birth, nickname,
                              gender, birth_date, birth_place, death_date, notes,
                              linked_user_id, photo_key, created_at, updated_at
                         FROM persons WHERE family_id = $1 ORDER BY id LIMIT $2"#,
                    family_id.into_uuid(),
                    lim
                )
                .fetch_all(&self.pool)
                .await
            }
            Some(c) => {
                sqlx::query_as!(
                    PersonRow,
                    r#"SELECT id, family_id, given_name, family_name, name_at_birth, nickname,
                              gender, birth_date, birth_place, death_date, notes,
                              linked_user_id, photo_key, created_at, updated_at
                         FROM persons WHERE family_id = $1 AND id > $2 ORDER BY id LIMIT $3"#,
                    family_id.into_uuid(),
                    c.into_uuid(),
                    lim
                )
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(|e| PersonRepoError::Db(e.to_string()))?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update(
        &self,
        family_id: FamilyId,
        id: PersonId,
        d: PersonDraft,
    ) -> Result<Person, PersonRepoError> {
        let res = sqlx::query_as!(
            PersonRow,
            r#"UPDATE persons SET
                 given_name=$3, family_name=$4, name_at_birth=$5, nickname=$6, gender=$7,
                 birth_date=$8, birth_place=$9, death_date=$10, notes=$11,
                 linked_user_id=$12
               WHERE family_id = $1 AND id = $2
               RETURNING id, family_id, given_name, family_name, name_at_birth, nickname,
                         gender, birth_date, birth_place, death_date, notes,
                         linked_user_id, photo_key, created_at, updated_at"#,
            family_id.into_uuid(),
            id.into_uuid(),
            d.given_name,
            d.family_name,
            d.name_at_birth,
            d.nickname,
            d.gender,
            d.birth_date,
            d.birth_place,
            d.death_date,
            d.notes,
            d.linked_user_id.map(UserId::into_uuid),
        )
        .fetch_optional(&self.pool)
        .await;
        match res {
            Ok(Some(r)) => Ok(r.into()),
            Ok(None) => Err(PersonRepoError::NotFound),
            Err(sqlx::Error::Database(db))
                if db.constraint() == Some("persons_family_id_linked_user_id_key") =>
            {
                Err(PersonRepoError::LinkedUserConflict)
            }
            Err(e) => Err(PersonRepoError::Db(e.to_string())),
        }
    }

    async fn delete(&self, family_id: FamilyId, id: PersonId) -> Result<(), PersonRepoError> {
        let res = sqlx::query!(
            "DELETE FROM persons WHERE family_id = $1 AND id = $2",
            family_id.into_uuid(),
            id.into_uuid()
        )
        .execute(&self.pool)
        .await
        .map_err(|e| PersonRepoError::Db(e.to_string()))?;
        if res.rows_affected() == 0 {
            return Err(PersonRepoError::NotFound);
        }
        Ok(())
    }

    async fn find_by_linked_user(
        &self,
        family_id: FamilyId,
        user_id: UserId,
    ) -> Result<Option<Person>, PersonRepoError> {
        let row = sqlx::query_as!(
            PersonRow,
            r#"SELECT id, family_id, given_name, family_name, name_at_birth, nickname,
                      gender, birth_date, birth_place, death_date, notes,
                      linked_user_id, photo_key, created_at, updated_at
                 FROM persons WHERE family_id = $1 AND linked_user_id = $2"#,
            family_id.into_uuid(),
            user_id.into_uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PersonRepoError::Db(e.to_string()))?;
        Ok(row.map(Into::into))
    }

    async fn set_linked_user_id(
        &self,
        family_id: FamilyId,
        id: PersonId,
        user_id: Option<UserId>,
    ) -> Result<(), PersonRepoError> {
        let res = sqlx::query!(
            r#"UPDATE persons SET linked_user_id = $3
                WHERE family_id = $1 AND id = $2"#,
            family_id.into_uuid(),
            id.into_uuid(),
            user_id.map(UserId::into_uuid),
        )
        .execute(&self.pool)
        .await;
        match res {
            Ok(out) => {
                if out.rows_affected() == 0 {
                    return Err(PersonRepoError::NotFound);
                }
                Ok(())
            }
            Err(sqlx::Error::Database(db))
                if db.constraint() == Some("persons_family_id_linked_user_id_key") =>
            {
                Err(PersonRepoError::LinkedUserConflict)
            }
            Err(e) => Err(PersonRepoError::Db(e.to_string())),
        }
    }

    async fn set_photo_key_for_linked_user(
        &self,
        user_id: UserId,
        photo_key: Option<String>,
    ) -> Result<u64, PersonRepoError> {
        // Cross-family broadcast — one statement, no per-family scoping.
        // The user's identity is the only thing that links these rows;
        // no family boundary applies (a user can be the "Werner" of one
        // family AND the "Werner" of another family they've joined).
        // Latest-write-wins: we overwrite whatever photo_key was there,
        // including individual person overrides. The simple semantic
        // Bent asked for ("without more logic").
        let res = sqlx::query!(
            "UPDATE persons SET photo_key = $2 WHERE linked_user_id = $1",
            user_id.into_uuid(),
            photo_key,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| PersonRepoError::Db(e.to_string()))?;
        Ok(res.rows_affected())
    }

    async fn set_photo_key(
        &self,
        family_id: FamilyId,
        id: PersonId,
        photo_key: Option<String>,
    ) -> Result<Option<String>, PersonRepoError> {
        // Single-statement swap returning the PREVIOUS photo_key. The CTE
        // materialises the value BEFORE the UPDATE executes — a naive
        // `RETURNING (SELECT photo_key FROM persons WHERE id=$2)` would
        // re-read the now-updated row and round-trip the caller's new key
        // back as the "previous" one.
        //
        // The caller best-effort-deletes the previous object from the store
        // after the commit, so a failed UPDATE never orphans the visible photo.
        let row = sqlx::query!(
            r#"WITH old AS (
                   SELECT photo_key FROM persons WHERE family_id = $1 AND id = $2
               )
               UPDATE persons SET photo_key = $3
                WHERE family_id = $1 AND id = $2
            RETURNING (SELECT photo_key FROM old) AS "previous""#,
            family_id.into_uuid(),
            id.into_uuid(),
            photo_key,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PersonRepoError::Db(e.to_string()))?;
        // `fetch_optional` returns None ONLY when the UPDATE's WHERE clause
        // matched zero rows.
        match row {
            None => Err(PersonRepoError::NotFound),
            Some(r) => Ok(r.previous),
        }
    }
}
