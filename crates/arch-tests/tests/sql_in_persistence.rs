//! SQL lives in the persistence crate only.

use std::fs;
use std::path::{Path, PathBuf};

use my_fam_tree_arch_tests::repo;
use my_fam_tree_arch_tests::sql::{EXCLUDED_CRATES, SqlHit, check_reexports, scan_source};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).into_iter().flatten().filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn crate_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(repo::root().join("crates"))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.join("Cargo.toml").is_file())
        .collect();
    dirs.sort();
    dirs
}

fn run(files: &[PathBuf], scan: fn(&str, &str) -> Result<Vec<SqlHit>, syn::Error>) -> Vec<String> {
    let root = repo::root();
    let mut out = Vec::new();
    for path in files {
        let name = path.strip_prefix(&root).unwrap_or(path).display().to_string();
        match fs::read_to_string(path).map(|src| scan(&name, &src)) {
            Ok(Ok(hits)) => {
                out.extend(hits.into_iter().map(|h| format!("{}:{}: {}", h.file, h.line, h.what)));
            }
            Ok(Err(e)) => out.push(format!("{name}: parse error: {e}")),
            Err(e) => out.push(format!("{name}: read error: {e}")),
        }
    }
    out
}

#[test]
fn sql_only_in_persistence() {
    let scanned: Vec<PathBuf> = crate_dirs()
        .into_iter()
        .filter(|d| {
            d.file_name().and_then(|n| n.to_str()).is_some_and(|n| !EXCLUDED_CRATES.contains(&n))
        })
        .collect();
    assert!(scanned.len() >= 8, "expected ≥ 8 scanned crates, found {scanned:?}");
    let mut files = Vec::new();
    for dir in &scanned {
        rust_files(&dir.join("src"), &mut files);
    }
    let hits = run(&files, scan_source);
    assert!(
        hits.is_empty(),
        "SQL outside crates/persistence ({} hits):\n{}",
        hits.len(),
        hits.join("\n")
    );
}

#[test]
fn persistence_does_not_reexport_sqlx() {
    let mut files = Vec::new();
    rust_files(&repo::root().join("crates/persistence/src"), &mut files);
    assert!(!files.is_empty(), "no persistence sources found");
    let hits = run(&files, check_reexports);
    assert!(hits.is_empty(), "persistence re-exports sqlx:\n{}", hits.join("\n"));
}
