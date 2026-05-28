//! The [`ObjectStore`] trait — single abstraction for both real S3 and the
//! local filesystem fallback. Keep this surface minimal: callers (the API
//! handlers + worker) only need to put/get/delete by key and produce
//! presigned URLs the browser can fetch directly.

use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;

use crate::StorageError;

#[async_trait]
pub trait ObjectStore: Send + Sync + std::fmt::Debug + 'static {
    /// Upload `bytes` under `key`, advertising `content_type` so a
    /// presigned download serves it back with the right headers.
    ///
    /// # Errors
    /// Returns [`StorageError::Backend`] on transport / SDK failures.
    async fn put(&self, key: &str, content_type: &str, bytes: Bytes) -> Result<(), StorageError>;

    /// Read the object back.
    ///
    /// # Errors
    /// Returns [`StorageError::NotFound`] when `key` has no current
    /// object — distinguishable from transport errors so api handlers
    /// can map it to 404 cleanly. Other failures surface as
    /// [`StorageError::Backend`].
    async fn get(&self, key: &str) -> Result<Bytes, StorageError>;

    /// Remove the object. Missing keys are NOT an error — delete is
    /// idempotent (mirrors `S3` + simplifies "delete on overwrite" flows).
    ///
    /// # Errors
    /// Returns [`StorageError::Backend`] on transport / SDK failures.
    async fn delete(&self, key: &str) -> Result<(), StorageError>;

    /// Time-bounded GET URL the browser can hit directly without
    /// proxying bytes through the api. For `S3` this is an `AWS` `SigV4`
    /// presigned URL; for the local backend it's a `/api/v1/uploads/{key}`
    /// route the api serves itself (the `expires_in` is informational
    /// because the local route doesn't enforce expiry).
    ///
    /// # Errors
    /// Returns [`StorageError::Backend`] if the presigner cannot be
    /// constructed (e.g. an out-of-range expiry).
    fn presigned_get(&self, key: &str, expires_in: Duration) -> Result<String, StorageError>;
}
