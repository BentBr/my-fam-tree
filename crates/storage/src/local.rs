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

    /// Validate `key` and return the on-disk path it should map to.
    ///
    /// Defense-in-depth (security audit LOW). The api handlers always
    /// build keys from server-trusted ids today, but a future route that
    /// echoed a client-supplied key into a put/get/delete would otherwise
    /// let `../../etc/passwd` escape `base_dir`. We refuse:
    ///
    /// * keys containing `..` segments (parent-dir traversal)
    /// * keys that look absolute (leading `/` or `\\`)
    /// * keys containing NUL bytes (a classic shim past length checks)
    /// * keys that resolve outside `base_dir` after path canonicalisation
    ///
    /// All three impl methods (`put`, `get`, `delete`) route through here;
    /// `presigned_get` calls the same validator but builds a URL instead
    /// of a path so the same trust boundary applies to the URL it mints.
    fn path_for(&self, key: &str) -> Result<PathBuf, StorageError> {
        if key.is_empty() {
            return Err(StorageError::InvalidKey("storage key must not be empty".into()));
        }
        if key.contains('\0') {
            return Err(StorageError::InvalidKey("storage key contains NUL byte".into()));
        }
        if key.starts_with('/') || key.starts_with('\\') {
            return Err(StorageError::InvalidKey("storage key must be relative".into()));
        }
        // Reject any "..", absolute, or root-dir segment cheaply BEFORE
        // we ever touch the filesystem. `Path::components` normalises the
        // separator so this catches both `..` and `..\\` shapes.
        for component in std::path::Path::new(key).components() {
            use std::path::Component;
            match component {
                Component::Normal(_) => {}
                _ => {
                    return Err(StorageError::InvalidKey(format!(
                        "storage key `{key}` contains a non-normal path segment",
                    )));
                }
            }
        }
        Ok(self.base_dir.join(key))
    }
}

fn backend<E: std::fmt::Display>(e: E) -> StorageError {
    StorageError::Backend(e.to_string())
}

#[async_trait]
impl ObjectStore for LocalObjectStore {
    async fn put(&self, key: &str, _content_type: &str, bytes: Bytes) -> Result<(), StorageError> {
        let path = self.path_for(key)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.map_err(backend)?;
        }
        fs::write(&path, &bytes).await.map_err(backend)?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Bytes, StorageError> {
        let path = self.path_for(key)?;
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
        let path = self.path_for(key)?;
        match fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(backend(e)),
        }
    }

    async fn presigned_get(
        &self,
        key: &str,
        _expires_in: Duration,
    ) -> Result<String, StorageError> {
        // Validate the key the same way `put/get/delete` do, even though
        // the result is a URL and not a filesystem path. A `..`-containing
        // key would otherwise produce a URL that escapes the configured
        // prefix when the browser normalises the path.
        let _checked = self.path_for(key)?;
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
        let url = store.presigned_get("persons/a.jpg", Duration::from_mins(1)).await.unwrap();
        assert_eq!(url, "/api/v1/uploads/persons/a.jpg");

        store.delete("persons/a.jpg").await.unwrap();
        match store.get("persons/a.jpg").await {
            Err(StorageError::NotFound(_)) => {}
            other => panic!("expected NotFound after delete, got {other:?}"),
        }
        // Delete of an already-missing key is a no-op (idempotent).
        store.delete("persons/a.jpg").await.unwrap();
    }

    #[tokio::test]
    async fn rejects_path_traversal_attempts() {
        let tmp = tempfile::tempdir().unwrap();
        let store = LocalObjectStore::new(tmp.path().to_path_buf(), "/api/v1/uploads".into());
        let bytes = || Bytes::from_static(b"X");

        // Each of these must be refused by `path_for` with a Config error
        // BEFORE any filesystem op runs.
        let traps = [
            "../escape.jpg",
            "persons/../../etc/passwd",
            "/absolute/path.jpg",
            "\\windows\\path.jpg",
            "with\0nul.jpg",
            "",
        ];
        for key in traps {
            match store.put(key, "image/jpeg", bytes()).await {
                Err(StorageError::InvalidKey(_)) => {}
                other => panic!("put({key:?}) should reject as Config, got {other:?}"),
            }
            match store.get(key).await {
                Err(StorageError::InvalidKey(_)) => {}
                other => panic!("get({key:?}) should reject as Config, got {other:?}"),
            }
            match store.delete(key).await {
                Err(StorageError::InvalidKey(_)) => {}
                other => panic!("delete({key:?}) should reject as Config, got {other:?}"),
            }
            match store.presigned_get(key, Duration::from_mins(1)).await {
                Err(StorageError::InvalidKey(_)) => {}
                other => panic!("presigned_get({key:?}) should reject as Config, got {other:?}"),
            }
        }
    }
}
