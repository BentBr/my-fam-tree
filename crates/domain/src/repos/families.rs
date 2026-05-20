use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{FamilyId, UserId};

#[derive(Debug, Clone)]
pub struct Family {
    pub id: FamilyId,
    pub name: String,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum FamilyRepoError {
    #[error("database: {0}")]
    Db(String),
    #[error("not found")]
    NotFound,
}

#[async_trait]
pub trait FamilyRepo: Send + Sync {
    async fn create(&self, name: &str, created_by: UserId) -> Result<Family, FamilyRepoError>;
    async fn find_by_id(&self, id: FamilyId) -> Result<Option<Family>, FamilyRepoError>;
    async fn rename(&self, id: FamilyId, name: &str) -> Result<(), FamilyRepoError>;
    async fn delete(&self, id: FamilyId) -> Result<(), FamilyRepoError>;
}
