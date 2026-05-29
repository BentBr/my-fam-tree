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
use my_family_domain::{PersonDraft, PersonId, PersonRepoError, Role};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

fn favourite_internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

use crate::auth::{require_role, user_claims_with_family};
use crate::response::{ApiResponse, Pagination};
use crate::services::audit;
use crate::validation::{string_too_long, value_required};
use crate::{ApiError, AppState, response_body};

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

/// Partial update. Every field is optional; only `Some(_)` fields overwrite
/// the corresponding column. An empty body is rejected as `validation.value_required`.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PersonUpdateReq {
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub name_at_birth: Option<String>,
    pub nickname: Option<String>,
    pub gender: Option<String>,
    pub birth_date: Option<NaiveDate>,
    pub birth_place: Option<String>,
    pub death_date: Option<NaiveDate>,
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
    object_store: &std::sync::Arc<dyn my_family_storage::ObjectStore>,
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
    p: my_family_domain::Person,
    object_store: &std::sync::Arc<dyn my_family_storage::ObjectStore>,
) -> PersonView {
    to_view_with_favourite(p, false, object_store).await
}

async fn to_view_with_favourite(
    p: my_family_domain::Person,
    is_favourite_for_me: bool,
    object_store: &std::sync::Arc<dyn my_family_storage::ObjectStore>,
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
        linked_user_id: p.linked_user_id.map(my_family_domain::UserId::into_uuid),
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
        linked_user_id: req.linked_user_id.map(my_family_domain::UserId::from_uuid),
    }
}

/// Apply a partial `PersonUpdateReq` on top of an existing person, returning
/// the resulting draft. None fields preserve the existing column.
fn merge_update(existing: &my_family_domain::Person, patch: PersonUpdateReq) -> PersonDraft {
    PersonDraft {
        given_name: patch.given_name.unwrap_or_else(|| existing.given_name.clone()),
        family_name: patch.family_name.unwrap_or_else(|| existing.family_name.clone()),
        name_at_birth: patch.name_at_birth.unwrap_or_else(|| existing.name_at_birth.clone()),
        nickname: patch.nickname.unwrap_or_else(|| existing.nickname.clone()),
        gender: patch.gender.unwrap_or_else(|| existing.gender.clone()),
        birth_date: patch.birth_date.or(existing.birth_date),
        birth_place: patch.birth_place.unwrap_or_else(|| existing.birth_place.clone()),
        death_date: patch.death_date.or(existing.death_date),
        notes: patch.notes.unwrap_or_else(|| existing.notes.clone()),
        linked_user_id: patch
            .linked_user_id
            .map(my_family_domain::UserId::from_uuid)
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
