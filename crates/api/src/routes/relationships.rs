//! `GET /api/v1/relationships` — full denormalized tree for the active family —
//! and `GET /api/v1/relationships/kinship` — the relations between two persons.

#![expect(
    clippy::future_not_send,
    reason = "actix handlers hold !Send request types (HttpRequest, Multipart); actix runs each worker on a single thread"
)]
#![expect(
    unreachable_pub,
    reason = "actix route macros turn each handler into a pub service struct"
)]

use actix_web::{HttpRequest, get, web};
use my_fam_tree_domain::{FamilyId, Person, PersonId};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::user_claims_with_family;
use crate::response::ApiResponse;
use crate::services::relationships_kinship::{KinshipPayload, build_kinship};
use crate::services::relationships_tree::{TreePayload, build_tree};
use crate::{ApiError, AppState, response_body, validation};

response_body!(pub TreePayloadResponseBody, TreePayload);
response_body!(pub KinshipPayloadResponseBody, KinshipPayload);

#[utoipa::path(
    get,
    path = "/api/v1/relationships",
    description = "Full denormalized family tree for the active family: every person with their \
parent / partner adjacency lists plus the raw parent and partner edges, in one payload the client \
renders without further round-trips.",
    responses(
        (status = 200, description = "Tree payload", body = TreePayloadResponseBody),
        (status = 401, description = "No session"),
    ),
    security(("cookie_access" = [])),
    tag = "relationships",
)]
#[get("/relationships")]
pub async fn tree(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<ApiResponse<TreePayload>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    let payload = build_tree(
        &state.persons,
        &state.parent_links,
        &state.partnerships,
        &state.favourites,
        &state.object_store,
        active.id,
        claims.user_id,
    )
    .await
    .map_err(ApiError::Internal)?;
    Ok(ApiResponse::ok(payload))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct KinshipQuery {
    /// Person A — the perspective the relations are described from.
    pub a: Uuid,
    /// Person B — the person every returned relation names.
    pub b: Uuid,
}

#[utoipa::path(
    get,
    path = "/api/v1/relationships/kinship",
    operation_id = "relationships_kinship",
    params(
        ("a" = Uuid, Query, description = "Person A — the perspective the relations are described from"),
        ("b" = Uuid, Query, description = "Person B — the person every returned relation names"),
    ),
    description = "Compute the kinship relations between two persons of the active family.

Relations come back as structured descriptors so the client renders localized labels itself (the \
server is locale-free). Order: blood relations first, then direct partners, then in-law and step \
relations — by distance within each group, so a cousin is listed before a spouse.

`up` / `down` count the generations from A up to the lowest common ancestor and back down to B: \
`(1,0)` is a parent, `(2,1)` an aunt or uncle, `(2,2)` a first cousin. `half`, `step` and `in_law` \
qualify the relation; `(0,0)` with a `partnership` is a direct partner. `path` lists the person ids \
from A to B for highlighting; `via_partnership_id` names the one partnership the path crosses.

Several relations can hold at once (a cousin who is also a sibling-in-law). Blood relations are \
reported once per lowest common ancestor and searched up to 32 generations; in-laws cross exactly \
one partnership that is open or ended by death. An empty `relations` array means the two persons \
are not related — a normal `200`, not an error.",
    responses(
        (status = 200, description = "The relations between the two persons (possibly empty)", body = KinshipPayloadResponseBody),
        (status = 400, description = "`a` or `b` is missing or not a UUID"),
        (status = 401, description = "No session"),
        (status = 404, description = "Either person is not in the active family"),
        (status = 422, description = "`a` and `b` are the same person, or no active family header"),
    ),
    security(("cookie_access" = [])),
    tag = "relationships",
)]
#[get("/relationships/kinship")]
pub async fn kinship(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<KinshipQuery>,
) -> Result<ApiResponse<KinshipPayload>, ApiError> {
    let (_claims, active) = user_claims_with_family(&req)?;
    if query.a == query.b {
        return Err(validation::must_differ("/b"));
    }
    let a = person_in_family(&state, active.id, query.a).await?;
    let b = person_in_family(&state, active.id, query.b).await?;
    let payload = build_kinship(&state.parent_links, &state.partnerships, active.id, &a, &b)
        .await
        .map_err(ApiError::Internal)?;
    Ok(ApiResponse::ok(payload))
}

/// Resolve `id` inside the active family; a person of another family is
/// indistinguishable from a missing one.
async fn person_in_family(
    state: &AppState,
    family_id: FamilyId,
    id: Uuid,
) -> Result<Person, ApiError> {
    state
        .persons
        .find_in_family(family_id, PersonId::from_uuid(id))
        .await
        .map_err(|e| ApiError::Internal(e.into()))?
        .ok_or(ApiError::PersonNotFound { id: Some(id) })
}
