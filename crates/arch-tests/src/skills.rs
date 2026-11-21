//! Repo paths referenced in `.claude/skills` and `.claude/agents` exist.

use std::path::Path;

use crate::docs::code_spans;
use crate::violation::Violation;

const DIR_PREFIXES: &[&str] = &[
    "crates/",
    "fe/",
    "scripts/",
    "migrations/",
    ".github/",
    ".claude/",
    ".githooks/",
    ".rusty_dev_tool/",
];
const ROOT_FILES: &[&str] = &["Cargo.toml", "compose.yaml", "deny.toml"];

/// Repo paths in inline code spans, with any `:<line>` suffix stripped.
/// Spans with placeholders, globs, variables or whitespace are skipped.
#[must_use]
pub fn path_refs(markdown: &str) -> Vec<String> {
    code_spans(markdown)
        .into_iter()
        .filter(|span| {
            !span.contains(['<', '>', '*', '{', '$']) && !span.contains(char::is_whitespace)
        })
        .map(|span| match span.split_once(':') {
            Some((path, line)) if line.chars().all(|c| c.is_ascii_digit() || c == '-') => path,
            _ => span,
        })
        .filter(|path| {
            DIR_PREFIXES.iter().any(|d| path.starts_with(d)) || ROOT_FILES.contains(path)
        })
        .map(str::to_owned)
        .collect()
}

/// Violations for referenced paths missing under `root`. `files` are
/// `(path, markdown)` pairs.
#[must_use]
pub fn check_refs(root: &Path, files: &[(String, String)]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|(file, md)| {
            path_refs(md).into_iter().filter(|p| !root.join(p).exists()).map(move |p| {
                Violation::new(file.clone(), format!("referenced path `{p}` does not exist"))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_repo_paths() {
        assert_eq!(
            path_refs("see `crates/api/src/state.rs:27` and `scripts/openapi.sh`"),
            vec!["crates/api/src/state.rs", "scripts/openapi.sh"],
            "paths with :line stripped"
        );
    }

    #[test]
    fn finds_root_files() {
        assert_eq!(
            path_refs("`Cargo.toml` and `deny.toml`"),
            vec!["Cargo.toml", "deny.toml"],
            "root files"
        );
    }

    #[test]
    fn ignores_placeholders_and_globs() {
        let md = "`crates/<name>/` `fe/src/**` `migrations/{a,b}` `crates/$X/src` `crates/api/src/x.rs --flag`";
        assert!(path_refs(md).is_empty(), "placeholders, globs, commands: {:?}", path_refs(md));
    }

    #[test]
    fn ignores_non_repo_code() {
        assert!(
            path_refs("`cargo test` `ApiError` `src/lib.rs` `Cargo.toml.bak`").is_empty(),
            "not repo paths"
        );
    }

    #[test]
    fn missing_path_is_violation() {
        let root = std::env::temp_dir().join(format!("skills-refs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert!(std::fs::create_dir_all(root.join("crates/api")).is_ok(), "mkdir");
        let files = vec![("s.md".to_owned(), "`crates/api` `crates/gone/src/x.rs`".to_owned())];
        let v = check_refs(&root, &files);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(
            v.iter().all(|x| x.file == "s.md" && x.detail.contains("crates/gone/src/x.rs")),
            "{v:?}"
        );
    }
}
