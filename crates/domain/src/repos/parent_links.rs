//! Parent-child link aggregate + repository trait.
//!
//! Edges are `(child_id, parent_id)` pairs scoped to a single family. A child
//! can have multiple parents (different `kind`s — biological, adoptive, …).
//! The repository's `insert` method must guarantee cycle prevention; see
//! `crate::relationships::would_create_cycle` for the in-memory check the
//! routes layer runs before calling through, and the persistence layer's
//! SERIALIZABLE transaction for the race-safe enforcement.

use async_trait::async_trait;

use crate::{FamilyId, PersonId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParentKind {
    Biological,
    Legal,
    Adoptive,
    Step,
    Social,
}

impl ParentKind {
    #[must_use]
    pub const fn as_db(self) -> &'static str {
        match self {
            Self::Biological => "biological",
            Self::Legal => "legal",
            Self::Adoptive => "adoptive",
            Self::Step => "step",
            Self::Social => "social",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParentLink {
    pub child_id: PersonId,
    pub parent_id: PersonId,
    pub kind: ParentKind,
    pub note: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ParentLinkRepoError {
    #[error("database: {0}")]
    Db(String),
    #[error("not found")]
    NotFound,
    #[error("cycle: parent is already a descendant of child")]
    Cycle,
    #[error("self-parent disallowed")]
    SelfParent,
    #[error("duplicate: edge (child_id, parent_id) already exists")]
    Duplicate,
}

#[async_trait]
pub trait ParentLinkRepo: Send + Sync {
    /// All `(child_id, parent_id)` edges for the family; used by cycle checks.
    async fn all_edges_in_family(
        &self,
        family_id: FamilyId,
    ) -> Result<Vec<(PersonId, PersonId)>, ParentLinkRepoError>;
    async fn list_for_family(
        &self,
        family_id: FamilyId,
    ) -> Result<Vec<ParentLink>, ParentLinkRepoError>;
    /// Insert a new parent link. Returns `Duplicate` when a row with the
    /// same `(child_id, parent_id)` already exists — the caller is
    /// expected to delete + re-create (or expose a `PATCH` for changing
    /// `kind`/`note`) rather than silently upserting. Implementations
    /// must perform the cycle check + insert atomically (SERIALIZABLE
    /// transaction) so concurrent inserts cannot bypass it.
    async fn insert(
        &self,
        family_id: FamilyId,
        child_id: PersonId,
        parent_id: PersonId,
        kind: ParentKind,
        note: &str,
    ) -> Result<(), ParentLinkRepoError>;
    async fn delete(
        &self,
        family_id: FamilyId,
        child_id: PersonId,
        parent_id: PersonId,
    ) -> Result<(), ParentLinkRepoError>;
}
