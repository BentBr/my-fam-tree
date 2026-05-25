//! `SELECT count(*)` helpers — used by integration tests and the seeder's
//! post-run assertions.
//!
//! The dynamic table name has to resolve to a `&'static str` at compile
//! time (sqlx 0.9's `SqlSafeStr` bound rejects ad-hoc `format!`s). The
//! enum below enumerates every table the rest of the workspace needs to
//! count, keeping all raw SQL inside this crate.

use sqlx::{PgPool, Row};

/// Tables the workspace needs row-counts for outside this crate.
#[derive(Debug, Clone, Copy)]
pub enum Table {
    Users,
    Families,
    FamilyMemberships,
    Persons,
    ParentLinks,
    Partnerships,
    PersonContacts,
    MagicLinkTokens,
}

impl Table {
    const fn count_sql(self) -> &'static str {
        match self {
            Self::Users => "SELECT count(*) FROM users",
            Self::Families => "SELECT count(*) FROM families",
            Self::FamilyMemberships => "SELECT count(*) FROM family_memberships",
            Self::Persons => "SELECT count(*) FROM persons",
            Self::ParentLinks => "SELECT count(*) FROM parent_links",
            Self::Partnerships => "SELECT count(*) FROM partnerships",
            Self::PersonContacts => "SELECT count(*) FROM person_contacts",
            Self::MagicLinkTokens => "SELECT count(*) FROM magic_link_tokens",
        }
    }
}

/// Count the rows in `table`.
///
/// # Errors
/// Propagates any underlying [`sqlx::Error`] from the count query.
pub async fn count_rows(pool: &PgPool, table: Table) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(table.count_sql()).fetch_one(pool).await?;
    Ok(row.get::<i64, _>(0))
}
