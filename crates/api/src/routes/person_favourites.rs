//! `PATCH /persons/{id}/favourite` — per-user favourite toggle.
//!
//! Favourites are **per-user, not per-person**: each `(user, person)`
//! row in `person_favourites` is the signed-in user's own private mark.
//! Two members of the same family see independent state on the same
//! person row.
//!
//! Authorization: any signed-in member of the active family may toggle
//! their own mark on ANY person in that family. The gate is "is this
//! person in my family" (resolved via `find_in_family`), NOT "do I have
//! edit rights" — favouriting reveals nothing and grants nothing; it
//! just bookmarks the row for the user's own projections (the tree star
//! and the `/upcoming?favourites_only=true` filter).
//!
//! The mutation is idempotent in both directions and is deliberately
//! NOT written to the audit log — a per-user preference is noise in the
//! family-wide event stream.

use actix_web::{HttpRequest, patch, web};
use my_fam_tree_domain::PersonId;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::user_claims_with_family;
use crate::response::ApiResponse;
use crate::{ApiError, AppState, response_body};

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FavouriteReq {
    pub is_favourite: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FavouriteRes {
    pub is_favourite: bool,
}

response_body!(pub PersonFavouriteResponseBody, FavouriteRes);

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/persons/{id}/favourite",
    operation_id = "persons_set_favourite",
    request_body = FavouriteReq,
    params(("id" = Uuid, Path, description = "Person id")),
    responses(
        (status = 200, description = "New favourite state", body = PersonFavouriteResponseBody),
        (status = 401, description = "No session"),
        (status = 404, description = "Not found in this family"),
    ),
    security(("cookie_access" = [])),
    tag = "persons",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[patch("/persons/{id}/favourite")]
pub async fn set_favourite(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    body: web::Json<FavouriteReq>,
) -> Result<ApiResponse<FavouriteRes>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    let id = path.into_inner();
    let person_id = PersonId::from_uuid(id);

    // Verify the person lives in the active family. Any member role may
    // toggle their own favourite mark on any person in the family.
    let _person = state
        .persons
        .find_in_family(active.id, person_id)
        .await
        .map_err(internal)?
        .ok_or(ApiError::PersonNotFound { id: Some(id) })?;

    let payload = body.into_inner();
    if payload.is_favourite {
        state.favourites.set(claims.user_id, person_id).await.map_err(internal)?;
    } else {
        state.favourites.unset(claims.user_id, person_id).await.map_err(internal)?;
    }
    Ok(ApiResponse::ok(FavouriteRes { is_favourite: payload.is_favourite }))
}
