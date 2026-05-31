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

use actix_web::{HttpRequest, delete, get, patch, post, web};
use chrono::NaiveDate;
use my_fam_tree_domain::{PersonDraft, PersonId, PersonRepoError, Role};
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

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
/// pub death_date: Option<Option<NaiveDate>>,
/// ```
///
/// Combined with `#[schema(value_type = Option<NaiveDate>, nullable)]`
/// so the `OpenAPI` surface stays unchanged.
//
// `clippy::option_option` flags the triple-state shape, but it IS the
// established serde pattern for "absent vs null vs value" PATCH bodies
// (the lint's "use a custom enum" suggestion would require also writing
// the serde glue every time). Localised allow rather than crate-wide so
// other accidental Option<Option<T>>s still fire.
#[allow(clippy::option_option)]
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

fn favourite_internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

use crate::auth::{require_role, user_claims_with_family};
use crate::response::{ApiResponse, Pagination};
use crate::services::audit;
use crate::validation::{string_too_long, value_required};
use crate::{ApiError, AppState, FieldViolation, response_body};

const PERSONS_LIST_DEFAULT_LIMIT: u32 = 50;
const PERSONS_LIST_MAX_LIMIT: u32 = 100;

// Free-text length caps (security audit MEDIUM). Without these the only
// upper bound is actix's default 32 KB JSON body, which lets an admin
// store hundreds of 30 KB notes blobs per person. Caps are in CHARACTERS
// (Unicode scalar values via `chars().count()`), not bytes — a German
// umlaut and an emoji each count as one.
const NAME_MAX: u32 = 200;
const SHORT_MAX: u32 = 100;
const NOTES_MAX: u32 = 2000;

/// Bound a single string field's character count. `None` paths (empty
/// strings) pass; callers handle required-vs-optional separately.
fn check_max(path: &str, value: &str, max: u32) -> Result<(), ApiError> {
    if u32::try_from(value.chars().count()).unwrap_or(u32::MAX) > max {
        return Err(string_too_long(path, max));
    }
    Ok(())
}

fn check_draft(d: &PersonDraft) -> Result<(), ApiError> {
    check_max("/given_name", &d.given_name, NAME_MAX)?;
    check_max("/family_name", &d.family_name, NAME_MAX)?;
    check_max("/name_at_birth", &d.name_at_birth, NAME_MAX)?;
    check_max("/nickname", &d.nickname, NAME_MAX)?;
    check_max("/gender", &d.gender, SHORT_MAX)?;
    check_max("/birth_place", &d.birth_place, SHORT_MAX)?;
    check_max("/notes", &d.notes, NOTES_MAX)?;
    Ok(())
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
    pub birth_date: Option<NaiveDate>,
    #[serde(default)]
    pub birth_place: String,
    pub death_date: Option<NaiveDate>,
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
// `Option<Option<NaiveDate>>` + `deserialize_optional_field` so a PATCH
// carrying `"death_date": null` clears the column rather than
// preserving the existing value. See the helper's doc for the wire-
// format vs handler-semantics mapping.
//
// `clippy::option_option` is silenced for the date fields here — see
// the comment on `deserialize_optional_field` for why.
#[allow(clippy::option_option)]
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PersonUpdateReq {
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub name_at_birth: Option<String>,
    pub nickname: Option<String>,
    pub gender: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    #[schema(value_type = Option<NaiveDate>, nullable)]
    pub birth_date: Option<Option<NaiveDate>>,
    pub birth_place: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    #[schema(value_type = Option<NaiveDate>, nullable)]
    pub death_date: Option<Option<NaiveDate>>,
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
    pub birth_date: Option<NaiveDate>,
    pub birth_place: String,
    pub death_date: Option<NaiveDate>,
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

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

/// Wall-clock TTL for the photo presigned URL. One hour balances "page
/// stays useful while the user reads" against "stale URL can't be reused
/// long-term if it leaks".
const PHOTO_URL_TTL: std::time::Duration = std::time::Duration::from_hours(1);

/// Resolve a stored `photo_key` to a fresh presigned URL. On storage
/// backend errors we log + return `None` so a single `MinIO` blip degrades
/// to "no photo shown" instead of a 500 on the whole person fetch.
///
/// Async because the S3 impl of `presigned_get` builds the URL via the
/// AWS SDK's `.presigned()` future. Calling it from a sync context inside
/// the actix arbiter panics ("can call blocking only when running on the
/// multi-threaded runtime").
async fn presigned_photo_url(
    object_store: &std::sync::Arc<dyn my_fam_tree_storage::ObjectStore>,
    key: Option<&str>,
) -> Option<String> {
    let key = key?;
    match object_store.presigned_get(key, PHOTO_URL_TTL).await {
        Ok(url) => Some(url),
        Err(e) => {
            tracing::warn!(error = ?e, photo_key = %key, "could not presign photo URL");
            None
        }
    }
}

async fn to_view(
    p: my_fam_tree_domain::Person,
    object_store: &std::sync::Arc<dyn my_fam_tree_storage::ObjectStore>,
) -> PersonView {
    to_view_with_favourite(p, false, object_store).await
}

async fn to_view_with_favourite(
    p: my_fam_tree_domain::Person,
    is_favourite_for_me: bool,
    object_store: &std::sync::Arc<dyn my_fam_tree_storage::ObjectStore>,
) -> PersonView {
    let photo_url = presigned_photo_url(object_store, p.photo_key.as_deref()).await;
    PersonView {
        id: p.id.into_uuid(),
        family_id: p.family_id.into_uuid(),
        given_name: p.given_name,
        family_name: p.family_name,
        name_at_birth: p.name_at_birth,
        nickname: p.nickname,
        gender: p.gender,
        birth_date: p.birth_date,
        birth_place: p.birth_place,
        death_date: p.death_date,
        notes: p.notes,
        linked_user_id: p.linked_user_id.map(my_fam_tree_domain::UserId::into_uuid),
        photo_url,
        is_favourite_for_me,
    }
}

fn draft_from_create(req: PersonCreateReq) -> PersonDraft {
    PersonDraft {
        given_name: req.given_name,
        family_name: req.family_name,
        name_at_birth: req.name_at_birth,
        nickname: req.nickname,
        gender: req.gender,
        birth_date: req.birth_date,
        birth_place: req.birth_place,
        death_date: req.death_date,
        notes: req.notes,
        linked_user_id: req.linked_user_id.map(my_fam_tree_domain::UserId::from_uuid),
    }
}

/// Apply a partial `PersonUpdateReq` on top of an existing person, returning
/// the resulting draft. None fields preserve the existing column.
///
/// Date fields use the triple-state convention from
/// [`deserialize_optional_field`]:
///   - outer `None` → field absent in the body → preserve existing
///   - outer `Some(None)` → field present and JSON-null → clear (write NULL)
///   - outer `Some(Some(v))` → field present with a value → set to `v`
fn merge_update(existing: &my_fam_tree_domain::Person, patch: PersonUpdateReq) -> PersonDraft {
    PersonDraft {
        given_name: patch.given_name.unwrap_or_else(|| existing.given_name.clone()),
        family_name: patch.family_name.unwrap_or_else(|| existing.family_name.clone()),
        name_at_birth: patch.name_at_birth.unwrap_or_else(|| existing.name_at_birth.clone()),
        nickname: patch.nickname.unwrap_or_else(|| existing.nickname.clone()),
        gender: patch.gender.unwrap_or_else(|| existing.gender.clone()),
        birth_date: patch.birth_date.unwrap_or(existing.birth_date),
        birth_place: patch.birth_place.unwrap_or_else(|| existing.birth_place.clone()),
        death_date: patch.death_date.unwrap_or(existing.death_date),
        notes: patch.notes.unwrap_or_else(|| existing.notes.clone()),
        linked_user_id: patch
            .linked_user_id
            .map(my_fam_tree_domain::UserId::from_uuid)
            .or(existing.linked_user_id),
    }
}

fn map_person_repo_err(e: PersonRepoError, id: Option<Uuid>) -> ApiError {
    match e {
        PersonRepoError::NotFound => ApiError::PersonNotFound { id },
        PersonRepoError::LinkedUserConflict => ApiError::ConflictStale,
        PersonRepoError::Db(_) => internal(e),
    }
}

/// Gate caller-supplied `linked_user_id` on CREATE / PATCH against the
/// consent model.
///
/// Linking a person row to a user account is privacy-sensitive: the
/// linked user inherits the row's contacts, photos, "this is you"
/// highlight in the tree, and several FE affordances. Without a consent
/// check, any admin in the family could silently bind any person row
/// to any `user_id` (their own user, an unrelated user, an attacker's
/// throwaway, …) just by posting `linked_user_id: <uuid>`.
///
/// The dedicated consent paths are:
/// * `POST /persons/{id}/claim` — caller self-links (we gate on the
///   role; the caller is provably acting for themselves).
/// * The invite-email round-trip — for linking *other* users, where
///   the linked party must click their own mailbox before the bind
///   happens.
///
/// CREATE and PATCH stay permissive in three specific cases (so the
/// FE can keep working without contortions):
/// 1. `proposed` is `None` — the caller didn't touch the field (most
///    edits) or sent an explicit null (PATCH's `Option<Uuid>` shape
///    can't distinguish absent from null, but both mean "no change").
/// 2. `proposed == current` — defensive echo of the existing value
///    from a FE that re-sends every field on save.
/// 3. `proposed == caller_user_id` AND `current` is `None` — admin
///    self-link of an UNLINKED row (e.g., the family-owner's first
///    row representing themselves on CREATE; or an admin staking
///    claim to a still-orphan row on PATCH). Equivalent to
///    `POST /claim` after the fact; allowed inline for convenience.
///
///    The `current.is_none()` guard is load-bearing for the
///    REASSIGN case: without it, an admin could PATCH an already-
///    linked row to themselves, bypassing the linked user's
///    consent (they should have had to use the invite flow). With
///    the guard, reassign-to-self requires the original linked
///    user to first clear their own link (via `DELETE /claim`
///    or an explicit PATCH `linked_user_id: null` from their
///    session).
///
/// Anything else (proposed is some *other* user's id, OR proposed
/// is self but the row is already linked to someone else) is rejected
/// with `validation.link_consent_required` so the FE can show a
/// targeted message and the admin understands they need the invite
/// flow.
fn check_link_consent(
    proposed: Option<my_fam_tree_domain::UserId>,
    current: Option<my_fam_tree_domain::UserId>,
    caller_user_id: my_fam_tree_domain::UserId,
) -> Result<(), ApiError> {
    match proposed {
        None => Ok(()),
        Some(p) if Some(p) == current => Ok(()),
        Some(p) if p == caller_user_id && current.is_none() => Ok(()),
        Some(_) => Err(ApiError::Validation(vec![FieldViolation::new(
            "/linked_user_id",
            "validation.link_consent_required",
            "Linking a person row to another user requires the invite-email \
             consent flow. Send an invite (POST /families/{id}/invites) and let \
             the recipient accept; or for self-link of an unlinked row, use \
             POST /persons/{id}/claim or set this field to your own user id.",
        )])),
    }
}

// ---------------------------------------------------------------------------
// GET /persons
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/persons",
    operation_id = "persons_list",
    params(
        ("cursor" = Option<Uuid>, Query, description = "Resume after this person id"),
        ("limit" = Option<u32>, Query, description = "Page size, 1..=100, default 50"),
    ),
    responses(
        (status = 200, description = "Family members page", body = PersonsListResponseBody),
        (status = 401, description = "No session"),
    ),
    security(("cookie_access" = [])),
    tag = "persons",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[get("/persons")]
pub async fn list(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<PersonsQuery>,
) -> Result<ApiResponse<Vec<PersonView>>, ApiError> {
    let (_claims, active) = user_claims_with_family(&req)?;
    let limit = query.limit.unwrap_or(PERSONS_LIST_DEFAULT_LIMIT).clamp(1, PERSONS_LIST_MAX_LIMIT);
    let cursor = query.cursor.map(PersonId::from_uuid);

    let rows = state
        .persons
        .list_for_family(active.id, cursor, limit)
        .await
        .map_err(|e| map_person_repo_err(e, None))?;

    // If we got `limit` items there *may* be more; surface the last id as the
    // next cursor. The caller polls again with `?cursor=<last>` until the
    // returned page is shorter than `limit`.
    let next_cursor = if u32::try_from(rows.len()).unwrap_or(u32::MAX) == limit {
        rows.last().map(|p| p.id.into_uuid().to_string())
    } else {
        None
    };
    let returned = u32::try_from(rows.len()).unwrap_or(u32::MAX);

    // `to_view` is async (it presigns the photo URL); collect the
    // per-person futures and resolve them in order.
    let mut views = Vec::with_capacity(rows.len());
    for p in rows {
        views.push(to_view(p, &state.object_store).await);
    }
    Ok(ApiResponse::page(views, Pagination { next_cursor, limit, returned }))
}

// ---------------------------------------------------------------------------
// POST /persons
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/persons",
    operation_id = "persons_create",
    request_body = PersonCreateReq,
    responses(
        (status = 200, description = "Person created", body = PersonViewResponseBody),
        (status = 401, description = "No session"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "persons",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/persons")]
pub async fn create(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<PersonCreateReq>,
) -> Result<ApiResponse<PersonView>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;

    let payload = body.into_inner();
    if payload.given_name.trim().is_empty() {
        return Err(value_required("/given_name"));
    }
    // Consent gate: CREATE may NOT pre-link the new row to anyone other
    // than the caller. See `check_link_consent` doc — a new row has no
    // existing link, so the only allowed Some(_) value is the caller's
    // own user id (admin self-link). Cross-user links go via invite.
    check_link_consent(
        payload.linked_user_id.map(my_fam_tree_domain::UserId::from_uuid),
        None,
        claims.user_id,
    )?;

    let draft = draft_from_create(payload);
    check_draft(&draft)?;
    let person =
        state.persons.create(active.id, draft).await.map_err(|e| map_person_repo_err(e, None))?;
    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "create",
        "person",
        Some(person.id.into_uuid()),
        serde_json::json!({}),
    )
    .await;
    Ok(ApiResponse::ok(to_view(person, &state.object_store).await))
}

// ---------------------------------------------------------------------------
// GET /persons/{id}
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/v1/persons/{id}",
    operation_id = "persons_get_one",
    params(("id" = Uuid, Path, description = "Person id")),
    responses(
        (status = 200, description = "Person", body = PersonViewResponseBody),
        (status = 401, description = "No session"),
        (status = 404, description = "Not found in this family"),
    ),
    security(("cookie_access" = [])),
    tag = "persons",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[get("/persons/{id}")]
pub async fn get_one(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<ApiResponse<PersonView>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    let id = path.into_inner();
    let person_id = PersonId::from_uuid(id);
    let person = state
        .persons
        .find_in_family(active.id, person_id)
        .await
        .map_err(|e| map_person_repo_err(e, Some(id)))?
        .ok_or(ApiError::PersonNotFound { id: Some(id) })?;
    let fav = state
        .favourites
        .is_favourite_for_user(claims.user_id, person_id)
        .await
        .map_err(favourite_internal)?;
    Ok(ApiResponse::ok(to_view_with_favourite(person, fav, &state.object_store).await))
}

// ---------------------------------------------------------------------------
// PATCH /persons/{id}
// ---------------------------------------------------------------------------

#[utoipa::path(
    patch,
    path = "/api/v1/persons/{id}",
    operation_id = "persons_update",
    request_body = PersonUpdateReq,
    params(("id" = Uuid, Path, description = "Person id")),
    responses(
        (status = 200, description = "Person updated", body = PersonViewResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Not allowed to edit this person"),
        (status = 404, description = "Not found in this family"),
        (status = 422, description = "Validation failed"),
    ),
    security(("cookie_access" = [])),
    tag = "persons",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[patch("/persons/{id}")]
pub async fn update(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<PersonUpdateReq>,
) -> Result<ApiResponse<PersonView>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    let id = path.into_inner();
    let person_id = PersonId::from_uuid(id);

    let existing = state
        .persons
        .find_in_family(active.id, person_id)
        .await
        .map_err(|e| map_person_repo_err(e, Some(id)))?
        .ok_or(ApiError::PersonNotFound { id: Some(id) })?;

    // Regular users may only edit the person row that maps to themselves.
    if active.role == Role::User && existing.linked_user_id != Some(claims.user_id) {
        return Err(ApiError::PersonNotEditable);
    }

    let payload = body.into_inner();
    // Reject a fully-empty PATCH so the caller knows nothing happened.
    let any_change = payload.given_name.is_some()
        || payload.family_name.is_some()
        || payload.name_at_birth.is_some()
        || payload.nickname.is_some()
        || payload.gender.is_some()
        || payload.birth_date.is_some()
        || payload.birth_place.is_some()
        || payload.death_date.is_some()
        || payload.notes.is_some()
        || payload.linked_user_id.is_some();
    if !any_change {
        return Err(value_required("/"));
    }
    if matches!(payload.given_name.as_deref(), Some(s) if s.trim().is_empty()) {
        return Err(value_required("/given_name"));
    }
    // Consent gate: PATCH may NOT change `linked_user_id` to anyone
    // other than the caller. The defensive-echo case (FE re-sends
    // the existing value as part of a wider edit) is allowed by
    // `check_link_consent`; the genuine change case routes through
    // POST /persons/{id}/claim (self) or the invite-email flow
    // (cross-user). See the helper's doc for the consent rationale.
    check_link_consent(
        payload.linked_user_id.map(my_fam_tree_domain::UserId::from_uuid),
        existing.linked_user_id,
        claims.user_id,
    )?;

    let draft = merge_update(&existing, payload);
    check_draft(&draft)?;
    let person = state
        .persons
        .update(active.id, person_id, draft)
        .await
        .map_err(|e| map_person_repo_err(e, Some(id)))?;
    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "update",
        "person",
        Some(id),
        serde_json::json!({}),
    )
    .await;
    Ok(ApiResponse::ok(to_view(person, &state.object_store).await))
}

// ---------------------------------------------------------------------------
// DELETE /persons/{id}
// ---------------------------------------------------------------------------

#[utoipa::path(
    delete,
    path = "/api/v1/persons/{id}",
    operation_id = "persons_delete",
    params(("id" = Uuid, Path, description = "Person id")),
    responses(
        (status = 200, description = "Person deleted", body = crate::response::NullResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Admin or owner required"),
        (status = 404, description = "Not found in this family"),
    ),
    security(("cookie_access" = [])),
    tag = "persons",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[delete("/persons/{id}")]
pub async fn delete(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<ApiResponse<serde_json::Value>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;
    let id = path.into_inner();
    state
        .persons
        .delete(active.id, PersonId::from_uuid(id))
        .await
        .map_err(|e| map_person_repo_err(e, Some(id)))?;
    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "delete",
        "person",
        Some(id),
        serde_json::json!({}),
    )
    .await;
    // Spec § 5: DELETE returns `{ "data": null }`, not a status string.
    Ok(ApiResponse::ok(serde_json::Value::Null))
}

// ---------------------------------------------------------------------------
// POST /persons/{id}/claim
// ---------------------------------------------------------------------------

/// Self-claim a person row.
///
/// The signed-in caller links `person.linked_user_id` to themselves. The
/// existing PATCH `/persons/{id}` already accepts an arbitrary
/// `linked_user_id` payload, but that path leaks consent: an admin can
/// silently bind any person row to any user with no notification on the
/// linked user's side. This dedicated endpoint is the consent-safe
/// variant — it only ever links the *caller*, never anyone else, and is
/// the back-end for the "Claim as me" button in `PersonDetail`.
///
/// Gated to admin / owner: regular `user`s onboard via the invite-email
/// round-trip (which doubles as cross-mailbox consent), so they don't need
/// this shortcut. Admins / owners who created a person row for themselves
/// can claim it in one click.
///
/// Rejects (409 [`ApiError::ConflictStale`]) when the person is already
/// linked to anyone (including the caller — a re-claim is a no-op rather
/// than an error worth surfacing to the user), or when the caller is
/// already linked to a different person in this family (the schema
/// enforces uniqueness `(family_id, linked_user_id)`; checking here gives
/// a clean error path instead of the bare
/// [`PersonRepoError::LinkedUserConflict`] mapping).
#[utoipa::path(
    post,
    path = "/api/v1/persons/{id}/claim",
    operation_id = "persons_claim",
    params(("id" = Uuid, Path, description = "Person id")),
    responses(
        (status = 200, description = "Person claimed by caller", body = PersonViewResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Admin or owner required"),
        (status = 404, description = "Not found in this family"),
        (status = 409,
         description = "Already linked, or caller already linked to another person in this family"),
    ),
    security(("cookie_access" = [])),
    tag = "persons",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/persons/{id}/claim")]
pub async fn claim(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<ApiResponse<PersonView>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;
    let id = path.into_inner();
    let person_id = PersonId::from_uuid(id);

    let existing = state
        .persons
        .find_in_family(active.id, person_id)
        .await
        .map_err(|e| map_person_repo_err(e, Some(id)))?
        .ok_or(ApiError::PersonNotFound { id: Some(id) })?;

    if existing.linked_user_id.is_some() {
        return Err(ApiError::ConflictStale);
    }

    if let Some(already) = state
        .persons
        .find_by_linked_user(active.id, claims.user_id)
        .await
        .map_err(|e| map_person_repo_err(e, None))?
    {
        tracing::info!(
            user_id = %claims.user_id.into_uuid(),
            existing_person_id = %already.id.into_uuid(),
            attempted_person_id = %id,
            "claim rejected: caller already linked to a different person in this family"
        );
        return Err(ApiError::ConflictStale);
    }

    state
        .persons
        .set_linked_user_id(active.id, person_id, Some(claims.user_id))
        .await
        .map_err(|e| map_person_repo_err(e, Some(id)))?;

    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "claim",
        "person",
        Some(id),
        serde_json::json!({}),
    )
    .await;

    // Re-fetch so the response reflects the new `linked_user_id`. We
    // can't reuse `existing` (it was the pre-claim snapshot), and
    // `set_linked_user_id` returns `()` so we don't get a row back from
    // the write itself.
    let updated = state
        .persons
        .find_in_family(active.id, person_id)
        .await
        .map_err(|e| map_person_repo_err(e, Some(id)))?
        .ok_or(ApiError::PersonNotFound { id: Some(id) })?;
    Ok(ApiResponse::ok(to_view(updated, &state.object_store).await))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use my_fam_tree_domain::UserId;
    use uuid::Uuid;

    use super::*;

    fn uid() -> UserId {
        UserId::from_uuid(Uuid::new_v4())
    }

    // The consent gate has a small but security-relevant truth table —
    // each branch gets its own test so a future refactor that drops a
    // branch fails loudly. Reviewers can read the test names as a spec.

    #[test]
    fn link_consent_allows_none_proposed() {
        // FE didn't touch the field (most edits) — always fine.
        let caller = uid();
        assert!(check_link_consent(None, None, caller).is_ok());
        assert!(check_link_consent(None, Some(uid()), caller).is_ok());
    }

    #[test]
    fn link_consent_allows_proposed_equal_to_current() {
        // Defensive echo — FE re-sends the existing value as part of a
        // wider edit. No change attempted, so the consent gate is moot.
        let caller = uid();
        let linked = uid();
        assert!(check_link_consent(Some(linked), Some(linked), caller).is_ok());
    }

    #[test]
    fn link_consent_allows_self_link_only_when_row_is_unlinked() {
        // Admin self-link at CREATE / PATCH of an UNLINKED row —
        // equivalent to POST /claim.
        let caller = uid();
        assert!(check_link_consent(Some(caller), None, caller).is_ok());
    }

    #[test]
    fn link_consent_rejects_admin_yanking_already_linked_row_to_self() {
        // The privacy bypass the consent gate is supposed to close:
        // an admin should NOT be able to silently take over a row
        // that's already linked to user A by PATCHing its
        // linked_user_id to themselves (PATCH would otherwise mimic
        // POST /claim without the original user's clearing step).
        // The `current.is_none()` guard makes this case fall through
        // to the rejection branch.
        let caller = uid();
        let other_user = uid();
        let err = check_link_consent(Some(caller), Some(other_user), caller)
            .expect_err("self-link over an existing link must require consent");
        match err {
            ApiError::Validation(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0].path, "/linked_user_id");
                assert_eq!(v[0].code, "validation.link_consent_required");
            }
            _ => panic!("expected Validation, got {err:?}"),
        }
    }

    #[test]
    fn link_consent_rejects_proposed_equal_to_other_user() {
        // The consent hole: admin tries to link a row to some OTHER
        // user's id without an invite round-trip. Must be rejected.
        let caller = uid();
        let other = uid();
        let err = check_link_consent(Some(other), None, caller).expect_err("must reject");
        match err {
            ApiError::Validation(v) => {
                assert_eq!(v.len(), 1);
                assert_eq!(v[0].path, "/linked_user_id");
                assert_eq!(v[0].code, "validation.link_consent_required");
            }
            _ => panic!("expected Validation, got {err:?}"),
        }
    }

    #[test]
    fn link_consent_rejects_admin_moving_link_between_other_users() {
        // Admin tries to silently re-bind from user A to user B
        // (neither is the caller). Must be rejected even though
        // there's already a link there.
        let caller = uid();
        let user_a = uid();
        let user_b = uid();
        assert!(check_link_consent(Some(user_b), Some(user_a), caller).is_err());
    }

    // ---------------------------------------------------------------------
    // PATCH /persons/{id} triple-state for date fields. Each row of the
    // table in `deserialize_optional_field` doc gets its own test so a
    // future refactor that collapses absent/null can't silently re-break
    // the "uncheck deceased" UI flow.
    // ---------------------------------------------------------------------

    fn fixture_person_with_death_date(date: Option<NaiveDate>) -> my_fam_tree_domain::Person {
        my_fam_tree_domain::Person {
            id: my_fam_tree_domain::PersonId::from_uuid(Uuid::new_v4()),
            family_id: my_fam_tree_domain::FamilyId::from_uuid(Uuid::new_v4()),
            given_name: "Alice".into(),
            family_name: String::new(),
            name_at_birth: String::new(),
            nickname: String::new(),
            gender: String::new(),
            birth_date: None,
            birth_place: String::new(),
            death_date: date,
            notes: String::new(),
            linked_user_id: None,
            photo_key: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn patch_with_absent_death_date_preserves_existing() {
        let existing_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let existing = fixture_person_with_death_date(Some(existing_date));
        // `{}` — field absent. `#[serde(default)]` produces `None`.
        let patch: PersonUpdateReq = serde_json::from_str(r#"{ "nickname": "X" }"#).unwrap();
        assert!(patch.death_date.is_none(), "absent field must deserialize to outer None");
        let draft = merge_update(&existing, patch);
        assert_eq!(draft.death_date, Some(existing_date), "absent must preserve");
    }

    #[test]
    fn patch_with_null_death_date_clears_existing() {
        // The split between absent and explicit-null is the whole point
        // of the triple-state pattern: a JSON `null` here deserializes
        // to outer `Some(None)`, which `merge_update` reads as "clear"
        // and writes NULL to the column.
        let existing_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let existing = fixture_person_with_death_date(Some(existing_date));
        let patch: PersonUpdateReq = serde_json::from_str(r#"{ "death_date": null }"#).unwrap();
        assert_eq!(patch.death_date, Some(None), "null must deserialize to outer Some(None)");
        let draft = merge_update(&existing, patch);
        assert_eq!(draft.death_date, None, "null must clear");
    }

    #[test]
    fn patch_with_value_death_date_sets_new_value() {
        let existing = fixture_person_with_death_date(None);
        let patch: PersonUpdateReq =
            serde_json::from_str(r#"{ "death_date": "2024-06-15" }"#).unwrap();
        let new_date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        assert_eq!(patch.death_date, Some(Some(new_date)), "value must deserialize to Some(Some)");
        let draft = merge_update(&existing, patch);
        assert_eq!(draft.death_date, Some(new_date), "value must set");
    }

    // Mirror the death_date trio for birth_date so the same shape is
    // pinned on both fields — they share the deserializer + merge path
    // and we don't want a future refactor to fix one and miss the other.

    #[test]
    fn patch_with_null_birth_date_clears_existing() {
        let mut existing = fixture_person_with_death_date(None);
        existing.birth_date = NaiveDate::from_ymd_opt(1990, 4, 12);
        let patch: PersonUpdateReq = serde_json::from_str(r#"{ "birth_date": null }"#).unwrap();
        let draft = merge_update(&existing, patch);
        assert_eq!(draft.birth_date, None);
    }
}
