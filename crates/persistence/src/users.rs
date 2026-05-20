//! Postgres-backed [`UserRepo`] implementation.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use my_family_domain::{Locale, User, UserId, UserRepo, UserRepoError};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct PgUserRepo {
    pool: PgPool,
}

impl PgUserRepo {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const fn locale_to_db(l: Locale) -> &'static str {
    l.as_str()
}

fn locale_from_db(s: &str) -> Locale {
    if s == "de" { Locale::De } else { Locale::En }
}

/// Mirror of the columns selected by every `users` query in this file.
/// Centralized so the row → `User` conversion lives in exactly one place.
#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    #[sqlx(rename = "email!")]
    email: String,
    display_name: String,
    #[sqlx(rename = "locale!")]
    locale: String,
    timezone: String,
    email_verified_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        Self {
            id: UserId::from_uuid(r.id),
            email: r.email,
            display_name: r.display_name,
            locale: locale_from_db(&r.locale),
            timezone: r.timezone,
            email_verified_at: r.email_verified_at,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl UserRepo for PgUserRepo {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, UserRepoError> {
        let row = sqlx::query_as!(
            UserRow,
            r#"SELECT id, email::text AS "email!", display_name, locale::text AS "locale!",
                      timezone, email_verified_at, created_at
                 FROM users WHERE email = $1"#,
            email
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserRepoError::Db(e.to_string()))?;
        Ok(row.map(Into::into))
    }

    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, UserRepoError> {
        let row = sqlx::query_as!(
            UserRow,
            r#"SELECT id, email::text AS "email!", display_name, locale::text AS "locale!",
                      timezone, email_verified_at, created_at
                 FROM users WHERE id = $1"#,
            id.into_uuid()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| UserRepoError::Db(e.to_string()))?;
        Ok(row.map(Into::into))
    }

    async fn create(&self, email: &str, locale: Locale) -> Result<User, UserRepoError> {
        let row = sqlx::query_as!(
            UserRow,
            r#"INSERT INTO users (email, locale) VALUES ($1, ($2::text)::user_locale)
               RETURNING id, email::text AS "email!", display_name, locale::text AS "locale!",
                         timezone, email_verified_at, created_at"#,
            email,
            locale_to_db(locale)
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| UserRepoError::Db(e.to_string()))?;
        Ok(row.into())
    }

    async fn mark_verified(&self, id: UserId) -> Result<(), UserRepoError> {
        sqlx::query!(
            "UPDATE users SET email_verified_at = COALESCE(email_verified_at, now()) WHERE id = $1",
            id.into_uuid()
        )
        .execute(&self.pool)
        .await
        .map_err(|e| UserRepoError::Db(e.to_string()))?;
        Ok(())
    }

    async fn update_locale(&self, id: UserId, locale: Locale) -> Result<(), UserRepoError> {
        sqlx::query!(
            "UPDATE users SET locale = ($2::text)::user_locale WHERE id = $1",
            id.into_uuid(),
            locale_to_db(locale)
        )
        .execute(&self.pool)
        .await
        .map_err(|e| UserRepoError::Db(e.to_string()))?;
        Ok(())
    }
}
