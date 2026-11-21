//! Locating the repository and its workspace members.

use std::path::PathBuf;

use cargo_metadata::MetadataCommand;

/// Repository root (`crates/arch-tests/../..`).
#[must_use]
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// Full `cargo metadata` for the workspace (dependencies resolved, so
/// dependency kinds and optional flags are available).
///
/// # Errors
/// When `cargo metadata` fails.
pub fn metadata() -> Result<cargo_metadata::Metadata, cargo_metadata::Error> {
    MetadataCommand::new().manifest_path(root().join("Cargo.toml")).no_deps().exec()
}

/// Names of all workspace member packages.
///
/// # Errors
/// When `cargo metadata` fails.
pub fn workspace_packages() -> Result<Vec<String>, cargo_metadata::Error> {
    let meta = metadata()?;
    Ok(meta.workspace_packages().iter().map(|p| p.name.to_string()).collect())
}
