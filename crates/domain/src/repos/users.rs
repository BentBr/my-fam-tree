use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::UserId;

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub display_name: String,
    pub locale: Locale,
    pub timezone: String,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    De,
}

impl Locale {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UserRepoError {
    #[error("database: {0}")]
    Db(String),
    #[error("not found")]
    NotFound,
}

#[async_trait]
pub trait UserRepo: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, UserRepoError>;
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, UserRepoError>;
    async fn create(&self, email: &str, locale: Locale) -> Result<User, UserRepoError>;
    async fn mark_verified(&self, id: UserId) -> Result<(), UserRepoError>;
    async fn update_locale(&self, id: UserId, locale: Locale) -> Result<(), UserRepoError>;
}
