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

use actix_web::{HttpRequest, delete, get, patch, post, web};
use chrono::NaiveDate;
use my_family_domain::{PersonDraft, PersonId, PersonRepoError, Role};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::{require_role, user_claims_with_family};
use crate::response::{ApiResponse, Pagination};
use crate::routes::persons_contact::{sync_email_from_linked_user, validate_contact_fields};
use crate::validation::value_required;
use crate::{ApiError, AppState, response_body};

const PERSONS_LIST_DEFAULT_LIMIT: u32 = 50;
const PERSONS_LIST_MAX_LIMIT: u32 = 100;

#[derive(Debug, Deserialize, ToSchema)]
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
    /// Contact email. Ignored on the wire when `linked_user_id` is set — the
    /// API overwrites the column with the linked user's email so the read
    /// path can stay simple (no JOIN).
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub street: String,
    #[serde(default)]
    pub house_number: String,
    #[serde(default)]
    pub zip: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub country: String,
    pub linked_user_id: Option<Uuid>,
}

/// Partial update. Every field is optional; only `Some(_)` fields overwrite
/// the corresponding column. An empty body is rejected as `validation.value_required`.
#[derive(Debug, Deserialize, ToSchema)]
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
    pub email: Option<String>,
    pub phone: Option<String>,
    pub street: Option<String>,
    pub house_number: Option<String>,
    pub zip: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
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
    pub email: String,
    pub phone: String,
    pub street: String,
    pub house_number: String,
    pub zip: String,
    pub city: String,
    pub country: String,
    pub linked_user_id: Option<Uuid>,
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

fn to_view(p: my_family_domain::Person) -> PersonView {
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
        email: p.email,
        phone: p.phone,
        street: p.street,
        house_number: p.house_number,
        zip: p.zip,
        city: p.city,
        country: p.country,
        linked_user_id: p.linked_user_id.map(my_family_domain::UserId::into_uuid),
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
        email: req.email,
        phone: req.phone,
        street: req.street,
        house_number: req.house_number,
        zip: req.zip,
        city: req.city,
        country: req.country,
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
        email: patch.email.unwrap_or_else(|| existing.email.clone()),
        phone: patch.phone.unwrap_or_else(|| existing.phone.clone()),
        street: patch.street.unwrap_or_else(|| existing.street.clone()),
        house_number: patch.house_number.unwrap_or_else(|| existing.house_number.clone()),
        zip: patch.zip.unwrap_or_else(|| existing.zip.clone()),
        city: patch.city.unwrap_or_else(|| existing.city.clone()),
        country: patch.country.unwrap_or_else(|| existing.country.clone()),
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

    Ok(ApiResponse::page(
        rows.into_iter().map(to_view).collect(),
        Pagination { next_cursor, limit, returned },
    ))
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
    let (_claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;

    let payload = body.into_inner();
    if payload.given_name.trim().is_empty() {
        return Err(value_required("/given_name"));
    }

    let mut draft = draft_from_create(payload);
    let email_overridden = sync_email_from_linked_user(&state, &mut draft).await?;
    validate_contact_fields(&draft, email_overridden)?;

    let person =
        state.persons.create(active.id, draft).await.map_err(|e| map_person_repo_err(e, None))?;
    Ok(ApiResponse::ok(to_view(person)))
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
    let (_claims, active) = user_claims_with_family(&req)?;
    let id = path.into_inner();
    let person = state
        .persons
        .find_in_family(active.id, PersonId::from_uuid(id))
        .await
        .map_err(|e| map_person_repo_err(e, Some(id)))?
        .ok_or(ApiError::PersonNotFound { id: Some(id) })?;
    Ok(ApiResponse::ok(to_view(person)))
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
        || payload.email.is_some()
        || payload.phone.is_some()
        || payload.street.is_some()
        || payload.house_number.is_some()
        || payload.zip.is_some()
        || payload.city.is_some()
        || payload.country.is_some()
        || payload.linked_user_id.is_some();
    if !any_change {
        return Err(value_required("/"));
    }
    if matches!(payload.given_name.as_deref(), Some(s) if s.trim().is_empty()) {
        return Err(value_required("/given_name"));
    }

    let mut draft = merge_update(&existing, payload);
    let email_overridden = sync_email_from_linked_user(&state, &mut draft).await?;
    validate_contact_fields(&draft, email_overridden)?;
    let person = state
        .persons
        .update(active.id, person_id, draft)
        .await
        .map_err(|e| map_person_repo_err(e, Some(id)))?;
    Ok(ApiResponse::ok(to_view(person)))
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
    let (_claims, active) = user_claims_with_family(&req)?;
    require_role(&active, Role::Admin)?;
    let id = path.into_inner();
    state
        .persons
        .delete(active.id, PersonId::from_uuid(id))
        .await
        .map_err(|e| map_person_repo_err(e, Some(id)))?;
    // Spec § 5: DELETE returns `{ "data": null }`, not a status string.
    Ok(ApiResponse::ok(serde_json::Value::Null))
}
