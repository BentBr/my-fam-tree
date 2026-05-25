//! Per-person contact rows for the dev seed.
//!
//! Phase 3 introduced the `person_contacts` table (one row per contact
//! method, with `kind`/`label`/`value`/`visibility`). The seeded family
//! gets a handful of realistic German contact rows attached to the same
//! three persons that previously carried flat contact columns:
//! - **Klaus** (linked to `admin@example.com`) — Work + Private email
//!   (Work is `admins_only` so the visibility filter for `user` role has
//!   a fixture to gate against; Private is `family`-visible and proves
//!   multi-entry-per-kind), mobile phone, home address.
//! - **Anna** (linked to `alice@example.com`) — email, mobile phone,
//!   shares address with Klaus.
//! - **Hannelore** (no linked user) — email, home phone, home address.
//!
//! Value shape per kind (named field, matches `routes::contacts`):
//! - `email`   → `{ "email":  "..." }`
//! - `phone`   → `{ "number": "..." }`
//! - `url`     → `{ "url":    "..." }`
//! - `other`   → `{ "text":   "..." }`
//! - `address` → `{ "street", "house_number", "zip", "city", "country" }`
//!
//! Rows are hardcoded with deterministic UUIDs so the seed is
//! idempotent — re-running upserts the same rows.

use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::ids::{SEED_PERSON_ANNA_ID, SEED_PERSON_HANNELORE_ID, SEED_PERSON_KLAUS_ID};

/// One contact row in the canonical seed.
struct ContactSeed {
    id: Uuid,
    person_id: Uuid,
    kind: &'static str,
    label: &'static str,
    value: Value,
    visibility: &'static str,
}

/// Number of contact rows seeded — surfaced for the test asserts.
///
/// 9 = the 8 original rows + Klaus's second email ("Private") which
/// demonstrates multi-entry-per-kind.
pub const SEED_CONTACT_COUNT: usize = 9;

/// Upsert every seeded contact row.
///
/// # Errors
/// Propagates any Postgres error from the upsert statements.
#[allow(clippy::too_many_lines, reason = "static table of 9 contacts; splitting hurts readability")]
pub async fn seed_contacts(pool: &PgPool) -> anyhow::Result<()> {
    let rows: [ContactSeed; SEED_CONTACT_COUNT] = [
        // Klaus — Work email (admins_only) + Private email (family).
        ContactSeed {
            id: Uuid::from_u128(0x0000_0005_0000_0000_0000_0000_0000_0001),
            person_id: SEED_PERSON_KLAUS_ID,
            kind: "email",
            label: "Work",
            value: json!({ "email": "admin@example.com" }),
            visibility: "admins_only",
        },
        ContactSeed {
            id: Uuid::from_u128(0x0000_0005_0000_0000_0000_0000_0000_0009),
            person_id: SEED_PERSON_KLAUS_ID,
            kind: "email",
            label: "Private",
            value: json!({ "email": "klaus.mueller@example.de" }),
            visibility: "family",
        },
        ContactSeed {
            id: Uuid::from_u128(0x0000_0005_0000_0000_0000_0000_0000_0002),
            person_id: SEED_PERSON_KLAUS_ID,
            kind: "phone",
            label: "Mobile",
            value: json!({ "number": "+49 40 5550101" }),
            visibility: "family",
        },
        ContactSeed {
            id: Uuid::from_u128(0x0000_0005_0000_0000_0000_0000_0000_0003),
            person_id: SEED_PERSON_KLAUS_ID,
            kind: "address",
            label: "Home",
            value: json!({
                "street": "Mittelweg",
                "house_number": "12",
                "zip": "20148",
                "city": "Hamburg",
                "country": "Deutschland",
            }),
            visibility: "family",
        },
        // Anna — family email (visible to everyone) + phone.
        ContactSeed {
            id: Uuid::from_u128(0x0000_0005_0000_0000_0000_0000_0000_0004),
            person_id: SEED_PERSON_ANNA_ID,
            kind: "email",
            label: "",
            value: json!({ "email": "alice@example.com" }),
            visibility: "family",
        },
        ContactSeed {
            id: Uuid::from_u128(0x0000_0005_0000_0000_0000_0000_0000_0005),
            person_id: SEED_PERSON_ANNA_ID,
            kind: "phone",
            label: "Mobile",
            value: json!({ "number": "+49 40 5550102" }),
            visibility: "family",
        },
        // Hannelore — no linked user, just contact info.
        ContactSeed {
            id: Uuid::from_u128(0x0000_0005_0000_0000_0000_0000_0000_0006),
            person_id: SEED_PERSON_HANNELORE_ID,
            kind: "email",
            label: "",
            value: json!({ "email": "hannelore.mueller@example.de" }),
            visibility: "family",
        },
        ContactSeed {
            id: Uuid::from_u128(0x0000_0005_0000_0000_0000_0000_0000_0007),
            person_id: SEED_PERSON_HANNELORE_ID,
            kind: "phone",
            label: "Home",
            value: json!({ "number": "+49 40 5550199" }),
            visibility: "family",
        },
        ContactSeed {
            id: Uuid::from_u128(0x0000_0005_0000_0000_0000_0000_0000_0008),
            person_id: SEED_PERSON_HANNELORE_ID,
            kind: "address",
            label: "Home",
            value: json!({
                "street": "Eppendorfer Landstraße",
                "house_number": "47",
                "zip": "20249",
                "city": "Hamburg",
                "country": "Deutschland",
            }),
            visibility: "family",
        },
    ];

    for c in rows {
        sqlx::query(
            "INSERT INTO person_contacts (id, person_id, kind, label, value, visibility) \
             VALUES ($1, $2, ($3::text)::contact_kind, $4, $5, ($6::text)::contact_visibility) \
             ON CONFLICT (id) DO UPDATE SET \
                 person_id = EXCLUDED.person_id, \
                 kind = EXCLUDED.kind, \
                 label = EXCLUDED.label, \
                 value = EXCLUDED.value, \
                 visibility = EXCLUDED.visibility",
        )
        .bind(c.id)
        .bind(c.person_id)
        .bind(c.kind)
        .bind(c.label)
        .bind(&c.value)
        .bind(c.visibility)
        .execute(pool)
        .await?;
    }
    Ok(())
}
