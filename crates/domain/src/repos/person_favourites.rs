//! Per-user `person_favourites` join + repository trait.
//!
//! Favourites are intentionally **private**: each `(user_id, person_id)`
//! row is the signed-in user's own mark. Two members of the same family
//! see independent state on the same person row. Toggling is idempotent
//! on both sides — `set` uses `ON CONFLICT DO NOTHING`; `unset` is a
//! plain DELETE that doesn't error when no row matched.
//!
//! `list_for_user` returns the user's full favourite set for a family
//! as a `HashSet<PersonId>`, which is what the tree + upcoming services
//! need: O(1) per-node membership checks while folding the projection.

use std::collections::HashSet;

use async_trait::async_trait;

use crate::{FamilyId, PersonId, UserId};

#[derive(Debug, thiserror::Error)]
pub enum PersonFavouriteRepoError {
    #[error("database: {0}")]
    Db(String),
}

#[async_trait]
pub trait PersonFavouriteRepo: Send + Sync {
    /// Idempotent insert. A no-op if the `(user, person)` pair already exists.
    async fn set(
        &self,
        user_id: UserId,
        person_id: PersonId,
    ) -> Result<(), PersonFavouriteRepoError>;

    /// Idempotent delete. A no-op if the `(user, person)` pair did not exist.
    async fn unset(
        &self,
        user_id: UserId,
        person_id: PersonId,
    ) -> Result<(), PersonFavouriteRepoError>;

    /// All persons the given user has favourited within `family_id`. Scoped
    /// to one family so cross-family favourites don't leak into projections.
    async fn list_for_user(
        &self,
        user_id: UserId,
        family_id: FamilyId,
    ) -> Result<HashSet<PersonId>, PersonFavouriteRepoError>;

    /// Single-pair convenience check. Cheaper than `list_for_user` when the
    /// caller only needs to know about one person.
    async fn is_favourite_for_user(
        &self,
        user_id: UserId,
        person_id: PersonId,
    ) -> Result<bool, PersonFavouriteRepoError>;
}
