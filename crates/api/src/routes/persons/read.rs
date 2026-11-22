//! `GET /persons` and `GET /persons/{id}`.

#![expect(
    clippy::future_not_send,
    reason = "actix handlers hold !Send request types (HttpRequest, Multipart); actix runs each worker on a single thread"
)]
#![expect(
    unreachable_pub,
    reason = "actix route macros turn each handler into a pub service struct"
)]

use actix_web::{HttpRequest, get, web};
use my_fam_tree_domain::PersonId;
use uuid::Uuid;

use super::support::{favourite_internal, map_person_repo_err, to_view, to_view_with_favourite};
use super::{PersonView, PersonViewResponseBody, PersonsListResponseBody, PersonsQuery};
use crate::auth::user_claims_with_family;
use crate::response::{ApiResponse, Pagination};
use crate::{ApiError, AppState};

const PERSONS_LIST_DEFAULT_LIMIT: u32 = 50;
const PERSONS_LIST_MAX_LIMIT: u32 = 100;

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
