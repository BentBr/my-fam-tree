//! `/persons` — CRUD for family members.
//!
//! All endpoints require an authenticated session **and** an active family
//! (`X-Family-Id` resolved from the JWT memberships). Role rules:
//! - `list`, `get_one`: any member of the active family.
//! - `create`, `delete`: admin or owner only.
//! - `update`: admin/owner can edit any person; a regular `user` may only
//!   edit the person row that has `linked_user_id = self`.
//!
//! `PATCH /persons/{id}` is a **partial** update — every `PersonUpdateReq`
//! field is optional and merges with the existing row server-side.
//!
//! Contact data (email / phone / address / url) lives in `person_contacts`,
//! not on this row — see [`crate::routes::contacts`].

mod read;
mod self_claim;
mod support;
mod write;

pub use read::{__path_get_one, __path_list, get_one, list};
pub use self_claim::{__path_claim, claim};
use serde::{Deserialize, Deserializer, Serialize};
use time::Date;
use utoipa::ToSchema;
use uuid::Uuid;
pub use write::{__path_create, __path_delete, __path_update, create, delete, update};

use crate::response_body;

/// Patch-style optional field deserializer that distinguishes "field
/// absent in the JSON body" from "field present and explicitly null".
///
/// Used on PATCH request fields that need three states:
///
/// | JSON body                | Wire reads as | Meaning                |
/// |--------------------------|---------------|------------------------|
/// | `{}` (field absent)      | `None`        | preserve existing      |
/// | `{ "death_date": null }` | `Some(None)`  | clear (write NULL)     |
/// | `{ "death_date": "..." }`| `Some(Some)`  | set to the given value |
///
/// Plain `Option<T>` collapses the first two cases into the same
/// `None`, so a handler can't tell "preserve" from "clear". Wrapping
/// in `Option<Option<T>>` + this deserializer keeps the wire format
/// identical (still `T | null | absent`) while letting the handler
/// distinguish the three cases.
///
/// Apply via:
///
/// ```ignore
/// #[serde(default, deserialize_with = "deserialize_optional_field")]
/// pub death_date: Option<Option<Date>>,
/// ```
///
/// Combined with `#[schema(value_type = Option<Date>, nullable)]`
/// so the `OpenAPI` surface stays unchanged.
//
// `clippy::option_option` flags the triple-state shape, but it IS the
// established serde pattern for "absent vs null vs value" PATCH bodies
// (the lint's "use a custom enum" suggestion would require also writing
// the serde glue every time). Localised allow rather than crate-wide so
// other accidental Option<Option<T>>s still fire.
#[expect(clippy::option_option, reason = "serde triple state for PATCH: absent vs null vs value")]
fn deserialize_optional_field<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    // `Option::<T>::deserialize` already maps JSON null → `None` and a
    // JSON value → `Some(T)`. Wrapping the result in an outer `Some(_)`
    // says "field was present"; serde's `#[serde(default)]` covers the
    // absent case by handing us a synthetic `None` instead of calling
    // this fn at all.
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PersonCreateReq {
    pub given_name: String,
    #[serde(default)]
    pub family_name: String,
    #[serde(default)]
    pub name_at_birth: String,
    #[serde(default)]
    pub nickname: String,
    #[serde(default)]
    pub gender: String,
    pub birth_date: Option<Date>,
    #[serde(default)]
    pub birth_place: String,
    pub death_date: Option<Date>,
    #[serde(default)]
    pub notes: String,
    pub linked_user_id: Option<Uuid>,
}

/// Partial update.
///
/// Every field is optional; only fields present in the request body
/// overwrite the corresponding column. An empty body is rejected as
/// `validation.value_required`. Send `null` on `birth_date` or
/// `death_date` to clear; send an empty string on string fields to
/// clear those.
//
// Implementation note (NOT exposed in OpenAPI — kept as a `//` comment
// so utoipa doesn't pull it into the schema description):
//
// `birth_date` and `death_date` use the triple-state pattern
// `Option<Option<Date>>` + `deserialize_optional_field` so a PATCH
// carrying `"death_date": null` clears the column rather than
// preserving the existing value. See the helper's doc for the wire-
// format vs handler-semantics mapping.
//
// `clippy::option_option` is silenced for the date fields here — see
// the comment on `deserialize_optional_field` for why.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PersonUpdateReq {
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub name_at_birth: Option<String>,
    pub nickname: Option<String>,
    pub gender: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    #[schema(value_type = Option<Date>, nullable)]
    pub birth_date: Option<Option<Date>>,
    pub birth_place: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    #[schema(value_type = Option<Date>, nullable)]
    pub death_date: Option<Option<Date>>,
    pub notes: Option<String>,
    pub linked_user_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PersonView {
    pub id: Uuid,
    pub family_id: Uuid,
    pub given_name: String,
    pub family_name: String,
    pub name_at_birth: String,
    pub nickname: String,
    pub gender: String,
    pub birth_date: Option<Date>,
    pub birth_place: String,
    pub death_date: Option<Date>,
    pub notes: String,
    pub linked_user_id: Option<Uuid>,
    /// Time-limited presigned URL for the person's photo, or `null` when
    /// the person has no photo. Re-presigned on every read; do NOT cache.
    pub photo_url: Option<String>,
    /// Per-user favourite mark for the signed-in caller. Always set on the
    /// single-person GET; the list endpoint leaves it `false` to avoid
    /// fanning N+1 favourite lookups across the page (the FE re-fetches the
    /// per-person GET when it opens the drawer).
    #[serde(default)]
    pub is_favourite_for_me: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PersonsQuery {
    pub cursor: Option<Uuid>,
    pub limit: Option<u32>,
}

response_body!(pub PersonViewResponseBody, PersonView);
response_body!(pub PersonsListResponseBody, Vec<PersonView>);
