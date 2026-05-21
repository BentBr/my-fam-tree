//! Partnership aggregate + repository trait.
//!
//! Each partnership row joins two persons in the same family. The DB-level
//! `CHECK (partner_a_id < partner_b_id)` enforces canonical pair ordering so
//! `(A, B)` and `(B, A)` cannot coexist; callers MUST canonicalize via
//! [`crate::relationships::canonicalize_pair`] before insert/update. The
//! `partnerships_unique_open` partial index dedupes currently-open
//! partnerships of the same `kind` for the same pair.

use async_trait::async_trait;
use chrono::NaiveDate;
use uuid::Uuid;

use crate::{FamilyId, PersonId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartnershipKind {
    Marriage,
    CivilUnion,
    Partnership,
}

impl PartnershipKind {
    #[must_use]
    pub const fn as_db(self) -> &'static str {
        match self {
            Self::Marriage => "marriage",
            Self::CivilUnion => "civil_union",
            Self::Partnership => "partnership",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartnershipEndReason {
    Divorce,
    Separation,
    Death,
}

impl PartnershipEndReason {
    #[must_use]
    pub const fn as_db(self) -> &'static str {
        match self {
            Self::Divorce => "divorce",
            Self::Separation => "separation",
            Self::Death => "death",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Partnership {
    pub id: Uuid,
    pub family_id: FamilyId,
    pub partner_a_id: PersonId,
    pub partner_b_id: PersonId,
    pub kind: PartnershipKind,
    pub started_on: Option<NaiveDate>,
    pub ended_on: Option<NaiveDate>,
    pub end_reason: Option<PartnershipEndReason>,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct PartnershipDraft {
    pub kind: PartnershipKind,
    pub started_on: Option<NaiveDate>,
    pub ended_on: Option<NaiveDate>,
    pub end_reason: Option<PartnershipEndReason>,
    pub note: String,
}

#[derive(Debug, thiserror::Error)]
pub enum PartnershipRepoError {
    #[error("database: {0}")]
    Db(String),
    #[error("not found")]
    NotFound,
    #[error("duplicate open partnership for this pair + kind")]
    Duplicate,
    #[error("partners must be in the same family")]
    CrossFamily,
}

#[async_trait]
pub trait PartnershipRepo: Send + Sync {
    async fn create(
        &self,
        family_id: FamilyId,
        a: PersonId,
        b: PersonId,
        draft: PartnershipDraft,
    ) -> Result<Partnership, PartnershipRepoError>;
    async fn list_for_family(
        &self,
        family_id: FamilyId,
    ) -> Result<Vec<Partnership>, PartnershipRepoError>;
    async fn update(
        &self,
        family_id: FamilyId,
        id: Uuid,
        draft: PartnershipDraft,
    ) -> Result<Partnership, PartnershipRepoError>;
    async fn delete(&self, family_id: FamilyId, id: Uuid) -> Result<(), PartnershipRepoError>;
}
