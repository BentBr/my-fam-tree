//! Person aggregate + repository trait.
//!
//! A `Person` is an individual within a single family. Each row may optionally
//! be linked to a `User` via `linked_user_id` (the FE shows a "claim profile"
//! affordance when a member's email matches an existing user). The
//! `(family_id, linked_user_id)` uniqueness guarantees a user is linked to at
//! most one person per family.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};

use crate::{FamilyId, PersonId, UserId};

#[derive(Debug, Clone)]
pub struct Person {
    pub id: PersonId,
    pub family_id: FamilyId,
    pub given_name: String,
    pub family_name: String,
    pub name_at_birth: String,
    pub nickname: String,
    pub gender: String,
    pub birth_date: Option<NaiveDate>,
    pub birth_place: String,
    pub death_date: Option<NaiveDate>,
    pub notes: String,
    pub linked_user_id: Option<UserId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input shape used by both `create` and `update`. Callers building partial
/// patches merge with the existing row server-side before calling `update`.
#[derive(Debug, Clone, Default)]
pub struct PersonDraft {
    pub given_name: String,
    pub family_name: String,
    pub name_at_birth: String,
    pub nickname: String,
    pub gender: String,
    pub birth_date: Option<NaiveDate>,
    pub birth_place: String,
    pub death_date: Option<NaiveDate>,
    pub notes: String,
    pub linked_user_id: Option<UserId>,
}

#[derive(Debug, thiserror::Error)]
pub enum PersonRepoError {
    #[error("database: {0}")]
    Db(String),
    #[error("not found")]
    NotFound,
    #[error("linked_user_id already in use for this family")]
    LinkedUserConflict,
}

#[async_trait]
pub trait PersonRepo: Send + Sync {
    async fn create(
        &self,
        family_id: FamilyId,
        draft: PersonDraft,
    ) -> Result<Person, PersonRepoError>;
    async fn find_in_family(
        &self,
        family_id: FamilyId,
        id: PersonId,
    ) -> Result<Option<Person>, PersonRepoError>;
    async fn list_for_family(
        &self,
        family_id: FamilyId,
        cursor: Option<PersonId>,
        limit: u32,
    ) -> Result<Vec<Person>, PersonRepoError>;
    async fn update(
        &self,
        family_id: FamilyId,
        id: PersonId,
        draft: PersonDraft,
    ) -> Result<Person, PersonRepoError>;
    async fn delete(&self, family_id: FamilyId, id: PersonId) -> Result<(), PersonRepoError>;
    async fn find_by_linked_user(
        &self,
        family_id: FamilyId,
        user_id: UserId,
    ) -> Result<Option<Person>, PersonRepoError>;
}
