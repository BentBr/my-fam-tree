//! Shared multipart parsing helpers for the upload endpoints.
//!
//! Both `/persons/{id}/photo` and `/users/me/avatar` (and any future
//! single-file upload route) need the same shape: drain a multipart
//! body, accept exactly one field named `file`, cap the raw bytes
//! BEFORE the image decoder gets them, and surface a clean
//! `ApiError::ImageInvalid` on every parse failure so the FE sees a
//! single error code regardless of where in the multipart pipeline the
//! bad bytes landed.
//!
//! Lifted out of `routes/person_photos.rs` + `routes/user_avatars.rs`
//! where the same function was duplicated word-for-word.

use actix_multipart::Multipart;
use bytes::{Bytes, BytesMut};
use futures_util::StreamExt;

use crate::ApiError;
use crate::images::MAX_RAW_BYTES;

/// Drain a single `file` field from a multipart body and return its
/// bytes plus the original filename (when the client included one).
///
/// Behaviour contract:
/// * Fields that are not literally named `file` are silently dropped.
///   The multipart spec allows any number of unrelated fields; we
///   refuse to fail the request on them.
/// * Exactly one `file` field. A second one → 422 (we don't want to
///   silently keep the first/last).
/// * Total raw bytes capped at [`MAX_RAW_BYTES`] (shared with
///   `images::validate_and_resize` so the per-chunk guard here and the
///   post-assembly guard inside the validator pin to the same number).
/// * Every error converts to [`ApiError::ImageInvalid`] so the route
///   surface stays single-code (`image.invalid`).
///
/// # Errors
/// Returns [`ApiError::ImageInvalid`] when the body has no `file`
/// field, has more than one, exceeds the size cap, or the multipart
/// transport itself errors mid-stream.
#[allow(clippy::future_not_send, reason = "actix Multipart is !Send by design")]
pub async fn read_single_file_field(
    mut payload: Multipart,
) -> Result<(Bytes, Option<String>), ApiError> {
    let mut bytes = BytesMut::with_capacity(64 * 1024);
    let mut filename: Option<String> = None;
    let mut seen_file = false;

    while let Some(field_res) = payload.next().await {
        let mut field =
            field_res.map_err(|e| ApiError::ImageInvalid { reason: format!("multipart: {e}") })?;
        if field.name().unwrap_or("") != "file" {
            // Drain unknown fields without ballooning memory; the policy
            // is "one file field named `file`" — anything else is ignored.
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
            if bytes.len().saturating_add(chunk.len()) > MAX_RAW_BYTES {
                return Err(ApiError::ImageInvalid {
                    reason: format!("upload exceeds maximum size of {MAX_RAW_BYTES} bytes"),
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
