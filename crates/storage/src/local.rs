//! `LocalObjectStore` — filesystem fallback for `cargo test` / dev runs
//! without `MinIO`. Writes under a configured `base_dir`. `presigned_get`
//! returns a `/api/v1/uploads/{key}`-style path the api can serve via a
//! plain file route (no expiry enforcement — informational only).

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use tokio::fs;
use tokio::io::AsyncReadExt;

use crate::{ObjectStore, StorageError};

#[derive(Debug, Clone)]
pub struct LocalObjectStore {
    base_dir: PathBuf,
    /// URL prefix used to build the "presigned" URL. E.g.
    /// `http://my-family.docker/api/v1/uploads` → the key is appended.
    url_prefix: String,
}

impl LocalObjectStore {
    #[must_use]
    pub const fn new(base_dir: PathBuf, url_prefix: String) -> Self {
        Self { base_dir, url_prefix }
    }

    fn path_for(&self, key: &str) -> PathBuf {
        // `key` is path-like ("persons/<uuid>.jpg") — join under the base.
        // We intentionally keep nested keys (e.g. "persons/xyz.jpg") working
        // by joining; the caller's keys are validated upstream.
        self.base_dir.join(key)
    }
}

fn backend<E: std::fmt::Display>(e: E) -> StorageError {
    StorageError::Backend(e.to_string())
}

#[async_trait]
impl ObjectStore for LocalObjectStore {
    async fn put(&self, key: &str, _content_type: &str, bytes: Bytes) -> Result<(), StorageError> {
        let path = self.path_for(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(backend)?;
        }
        fs::write(&path, &bytes).await.map_err(backend)?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Bytes, StorageError> {
        let path = self.path_for(key);
        let mut file = match fs::File::open(&path).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(StorageError::NotFound(key.to_string()));
            }
            Err(e) => return Err(backend(e)),
        };
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).await.map_err(backend)?;
        Ok(Bytes::from(buf))
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let path = self.path_for(key);
        match fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(backend(e)),
        }
    }

    fn presigned_get(&self, key: &str, _expires_in: Duration) -> Result<String, StorageError> {
        Ok(format!("{}/{}", self.url_prefix.trim_end_matches('/'), key))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[tokio::test]
    async fn put_get_delete_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let store = LocalObjectStore::new(tmp.path().to_path_buf(), "/api/v1/uploads".into());

        store.put("persons/a.jpg", "image/jpeg", Bytes::from_static(b"PHOTO")).await.unwrap();
        let got = store.get("persons/a.jpg").await.unwrap();
        assert_eq!(&got[..], b"PHOTO");
        let url = store.presigned_get("persons/a.jpg", Duration::from_mins(1)).unwrap();
        assert_eq!(url, "/api/v1/uploads/persons/a.jpg");

        store.delete("persons/a.jpg").await.unwrap();
        match store.get("persons/a.jpg").await {
            Err(StorageError::NotFound(_)) => {}
            other => panic!("expected NotFound after delete, got {other:?}"),
        }
        // Delete of an already-missing key is a no-op (idempotent).
        store.delete("persons/a.jpg").await.unwrap();
    }
}
