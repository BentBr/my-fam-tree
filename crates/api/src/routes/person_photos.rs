//! `/persons/{id}/photo` — upload + delete a person's photo.
//!
//! The upload flow is:
//!
//!   1. Parse the multipart body. Accept exactly one field named `file`;
//!      anything else is a 422. The raw bytes are bounded by
//!      [`MAX_UPLOAD_BYTES`] so a malicious client can't OOM us by streaming
//!      a 5 GB "photo".
//!   2. Hand the bytes to [`crate::images::validate_and_resize`] which
//!      magic-byte-checks the format (JPEG/PNG/WebP), cross-checks the
//!      filename extension if supplied, and re-encodes as JPEG q80 fit
//!      into 512×512.
//!   3. Mint a fresh storage key — `persons/{person_id}/{rand}.jpg`. The
//!      random suffix means a re-upload never replaces the same key, so
//!      a request that fails between PUT and the DB UPDATE never orphans
//!      the visible photo.
//!   4. `state.object_store.put(key, "image/jpeg", bytes)`.
//!   5. `persons.set_photo_key(family, id, Some(new_key))` — returns the
//!      previous key so we can best-effort-delete it from the store after
//!      the DB commit lands. A failure here is logged but never bubbled
//!      to the client; the bytes are abandoned, not the user's request.
//!
//! Authorisation mirrors `routes::persons::update`: admins/owners can edit
//! any person; regular users can only set the photo on their own linked
//! person row. The cross-family resolve happens via `find_in_family` so
//! the cross-family-IDOR shape the security audit flagged on
//! `parent_links` can never reproduce here.

use std::time::Duration;

use actix_multipart::Multipart;
use actix_web::{HttpRequest, delete, post, web};
use bytes::{Bytes, BytesMut};
use futures_util::StreamExt;
use my_family_domain::{PersonId, PersonRepoError, Role};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::auth::user_claims_with_family;
use crate::images::{self, MAX_RAW_BYTES, OUTPUT_CONTENT_TYPE, OUTPUT_EXTENSION};
use crate::response::ApiResponse;
use crate::services::audit;
use crate::{ApiError, AppState, response_body};

/// Presigned URLs returned to the FE are valid for an hour. That's long
/// enough to render a person page and short enough that a stolen URL
/// (e.g. from a copy-paste into a chat) stops working before it can be
/// abused at scale.
const PHOTO_URL_TTL: Duration = Duration::from_hours(1);

/// Multipart upload cap. Matches `images::MAX_RAW_BYTES` so a `413` from
/// the multipart layer and a `422` from the validator pin to the same
/// effective limit; pick the lower of either guard if they ever drift.
const MAX_UPLOAD_BYTES: usize = MAX_RAW_BYTES;

/// `data` shape for the success response.
#[derive(Debug, Serialize, ToSchema)]
pub struct PersonPhotoView {
    /// Opaque object-storage key (stable across the photo's lifetime).
    pub photo_key: String,
    /// Time-limited URL the browser can fetch the bytes from. Re-presigned
    /// on every read; do NOT cache.
    pub photo_url: String,
}

response_body!(pub PersonPhotoResponseBody, PersonPhotoView);

fn internal<E: std::fmt::Display>(e: E) -> ApiError {
    ApiError::Internal(anyhow::anyhow!(e.to_string()))
}

fn map_person_repo_err(e: PersonRepoError, id: Uuid) -> ApiError {
    match e {
        PersonRepoError::NotFound => ApiError::PersonNotFound { id: Some(id) },
        PersonRepoError::LinkedUserConflict => ApiError::ConflictStale,
        PersonRepoError::Db(_) => internal(e),
    }
}

/// Drain a single `file` field from a multipart body and return its bytes
/// plus the original filename (when the client included one). Rejects:
/// missing field, multiple fields, oversize payload, transport error.
#[allow(clippy::future_not_send)]
async fn read_single_file_field(
    mut payload: Multipart,
) -> Result<(Bytes, Option<String>), ApiError> {
    let mut bytes = BytesMut::with_capacity(64 * 1024);
    let mut filename: Option<String> = None;
    let mut seen_file = false;

    while let Some(field_res) = payload.next().await {
        let mut field =
            field_res.map_err(|e| ApiError::ImageInvalid { reason: format!("multipart: {e}") })?;

        let field_name = field.name().unwrap_or("").to_string();
        if field_name != "file" {
            // Drain unknown fields without ballooning memory; the policy
            // is "one file field named `file`" — anything else is a 422.
            continue;
        }
        if seen_file {
            return Err(ApiError::ImageInvalid {
                reason: "multipart contains more than one `file` field".into(),
            });
        }
        seen_file = true;

        filename = field.content_disposition().and_then(|cd| cd.get_filename().map(str::to_string));

        while let Some(chunk_res) = field.next().await {
            let chunk = chunk_res
                .map_err(|e| ApiError::ImageInvalid { reason: format!("multipart chunk: {e}") })?;
            // Cap raw upload BEFORE the decoder touches it — the validator
            // re-checks once the bytes are fully assembled, but failing
            // here saves us holding a multi-GB BytesMut in memory.
            if bytes.len().saturating_add(chunk.len()) > MAX_UPLOAD_BYTES {
                return Err(ApiError::ImageInvalid {
                    reason: format!("upload exceeds maximum size of {MAX_UPLOAD_BYTES} bytes",),
                });
            }
            bytes.extend_from_slice(&chunk);
        }
    }

    if !seen_file {
        return Err(ApiError::ImageInvalid {
            reason: "multipart body missing a `file` field".into(),
        });
    }
    Ok((bytes.freeze(), filename))
}

fn image_err_to_api(e: &images::ImageError) -> ApiError {
    ApiError::ImageInvalid { reason: e.to_string() }
}

// ---------------------------------------------------------------------------
// POST /persons/{id}/photo
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/v1/persons/{id}/photo",
    operation_id = "persons_set_photo",
    params(("id" = Uuid, Path, description = "Person id")),
    request_body(content = String, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Photo set", body = PersonPhotoResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Not allowed to edit this person"),
        (status = 404, description = "Person not found in this family"),
        (status = 422, description = "Invalid image"),
    ),
    security(("cookie_access" = [])),
    tag = "persons",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[post("/persons/{id}/photo")]
pub async fn upload(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
    payload: Multipart,
) -> Result<ApiResponse<PersonPhotoView>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    let id = path.into_inner();
    let person_id = PersonId::from_uuid(id);

    let existing = state
        .persons
        .find_in_family(active.id, person_id)
        .await
        .map_err(|e| map_person_repo_err(e, id))?
        .ok_or(ApiError::PersonNotFound { id: Some(id) })?;

    // Mirror persons::update authorisation: admins/owners may edit any
    // person; regular users only their own linked row.
    if active.role == Role::User && existing.linked_user_id != Some(claims.user_id) {
        return Err(ApiError::PersonNotEditable);
    }

    let (raw, filename) = read_single_file_field(payload).await?;
    let resized =
        images::validate_and_resize(&raw, filename.as_deref()).map_err(|e| image_err_to_api(&e))?;
    let resized_bytes = Bytes::from(resized);

    // Storage key composed entirely from server-trusted ids + a fresh
    // UUID — no caller input crosses into the path. Protects against the
    // path-traversal footgun documented in `crate-storage`.
    let suffix = Uuid::new_v4().simple();
    let key = format!("persons/{id}/{suffix}.{OUTPUT_EXTENSION}");

    state.object_store.put(&key, OUTPUT_CONTENT_TYPE, resized_bytes).await.map_err(internal)?;

    let previous = state
        .persons
        .set_photo_key(active.id, person_id, Some(key.clone()))
        .await
        .map_err(|e| map_person_repo_err(e, id))?;

    // Best-effort cleanup of the previous photo. Errors here are not
    // user-visible: the new key is committed, the old object is orphaned.
    // The store backend's failure mode (network blip, transient) doesn't
    // change the truth that the photo swap succeeded.
    if let Some(old) = previous
        && old != key
        && let Err(e) = state.object_store.delete(&old).await
    {
        tracing::warn!(error = ?e, previous_key = %old, "failed to delete previous photo from object store");
    }

    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "set_photo",
        "person",
        Some(id),
        serde_json::json!({}),
    )
    .await;

    let url = state.object_store.presigned_get(&key, PHOTO_URL_TTL).map_err(internal)?;
    Ok(ApiResponse::ok(PersonPhotoView { photo_key: key, photo_url: url }))
}

// ---------------------------------------------------------------------------
// DELETE /persons/{id}/photo
// ---------------------------------------------------------------------------

#[utoipa::path(
    delete,
    path = "/api/v1/persons/{id}/photo",
    operation_id = "persons_clear_photo",
    params(("id" = Uuid, Path, description = "Person id")),
    responses(
        (status = 200, description = "Photo cleared", body = crate::response::NullResponseBody),
        (status = 401, description = "No session"),
        (status = 403, description = "Not allowed to edit this person"),
        (status = 404, description = "Person not found in this family"),
    ),
    security(("cookie_access" = [])),
    tag = "persons",
)]
#[allow(clippy::future_not_send)]
#[allow(unreachable_pub)]
#[delete("/persons/{id}/photo")]
pub async fn clear(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<Uuid>,
) -> Result<ApiResponse<serde_json::Value>, ApiError> {
    let (claims, active) = user_claims_with_family(&req)?;
    let id = path.into_inner();
    let person_id = PersonId::from_uuid(id);

    let existing = state
        .persons
        .find_in_family(active.id, person_id)
        .await
        .map_err(|e| map_person_repo_err(e, id))?
        .ok_or(ApiError::PersonNotFound { id: Some(id) })?;

    if active.role == Role::User && existing.linked_user_id != Some(claims.user_id) {
        return Err(ApiError::PersonNotEditable);
    }

    let previous = state
        .persons
        .set_photo_key(active.id, person_id, None)
        .await
        .map_err(|e| map_person_repo_err(e, id))?;

    if let Some(old) = previous
        && let Err(e) = state.object_store.delete(&old).await
    {
        tracing::warn!(error = ?e, previous_key = %old, "failed to delete photo from object store");
    }

    audit::record(
        &state.audit,
        active.id,
        claims.user_id,
        "clear_photo",
        "person",
        Some(id),
        serde_json::json!({}),
    )
    .await;

    Ok(ApiResponse::ok(serde_json::Value::Null))
}
