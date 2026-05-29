//! Storage error model — one enum across both S3 and local backends.

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The requested object key doesn't exist.
    #[error("object not found: {0}")]
    NotFound(String),
    /// Transport / IO / SDK failure. The string carries the underlying
    /// error message for tracing; we don't surface SDK types to callers
    /// so swapping backends doesn't ripple through the API.
    #[error("{0}")]
    Backend(String),
    /// Caller passed an invalid object key (empty, path-traversing,
    /// absolute, or containing NUL bytes). The path-traversal-guard in
    /// `LocalObjectStore::path_for` returns this BEFORE touching the
    /// filesystem; handlers map it to a 422.
    #[error("invalid storage key: {0}")]
    InvalidKey(String),
}
