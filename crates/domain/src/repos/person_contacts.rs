//! `person_contacts` aggregate + repository trait.
//!
//! Per-person contact rows replace the flat email/phone/address columns
//! that lived directly on `persons`. Each row carries a `kind`
//! discriminator (`email`, `phone`, `address`, `url`, `other`), a
//! structured JSONB `value`, an optional human `label`, and a
//! visibility enum that the API layer enforces on read.

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::PersonId;

/// Discriminator for the contact row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactKind {
    Email,
    Phone,
    Address,
    Url,
    Other,
}

impl ContactKind {
    /// Lower-case DB representation, matching the `contact_kind` enum
    /// in `migrations/0005_contacts_and_audit.sql`.
    #[must_use]
    pub const fn as_db(self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::Phone => "phone",
            Self::Address => "address",
            Self::Url => "url",
            Self::Other => "other",
        }
    }
}

/// Read-time gate on a contact. `Family` means every member of the
/// owning family may see it; `AdminsOnly` hides the row from `user`
/// role members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactVisibility {
    Family,
    AdminsOnly,
}

impl ContactVisibility {
    /// Lower-snake-case DB representation, matching the
    /// `contact_visibility` enum.
    #[must_use]
    pub const fn as_db(self) -> &'static str {
        match self {
            Self::Family => "family",
            Self::AdminsOnly => "admins_only",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Contact {
    pub id: Uuid,
    pub person_id: PersonId,
    pub kind: ContactKind,
    pub label: String,
    pub value: Value,
    pub visibility: ContactVisibility,
}

#[derive(Debug, Clone)]
pub struct ContactDraft {
    pub kind: ContactKind,
    pub label: String,
    pub value: Value,
    pub visibility: ContactVisibility,
}

#[derive(Debug, thiserror::Error)]
pub enum ContactRepoError {
    #[error("database: {0}")]
    Db(String),
    #[error("not found")]
    NotFound,
}

#[async_trait]
pub trait PersonContactRepo: Send + Sync {
    async fn list_for_person(&self, person_id: PersonId) -> Result<Vec<Contact>, ContactRepoError>;
    async fn create(
        &self,
        person_id: PersonId,
        draft: ContactDraft,
    ) -> Result<Contact, ContactRepoError>;
    async fn update(&self, id: Uuid, draft: ContactDraft) -> Result<Contact, ContactRepoError>;
    async fn delete(&self, id: Uuid) -> Result<(), ContactRepoError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Contact>, ContactRepoError>;
}
