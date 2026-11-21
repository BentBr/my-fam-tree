//! Repo paths referenced in `.claude/skills/**/*.md`, `.claude/agents/*.md` and
//! `CLAUDE.md` exist, so skills never point Claude at moved or deleted files.

use std::fs;
use std::path::{Path, PathBuf};

use my_fam_tree_arch_tests::repo;
use my_fam_tree_arch_tests::skills::check_refs;
use my_fam_tree_arch_tests::violation::report;

fn markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for path in fs::read_dir(dir).into_iter().flatten().filter_map(Result::ok).map(|e| e.path()) {
        if path.is_dir() {
            markdown_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}

fn skill_and_agent_docs() -> Vec<(String, String)> {
    let root = repo::root();
    let mut paths = Vec::new();
    markdown_files(&root.join(".claude/skills"), &mut paths);
    markdown_files(&root.join(".claude/agents"), &mut paths);
    paths.push(root.join("CLAUDE.md"));
    paths.sort();
    paths
        .iter()
        .filter_map(|p| {
            let name = p.strip_prefix(&root).unwrap_or(p).display().to_string();
            fs::read_to_string(p).ok().map(|md| (name, md))
        })
        .collect()
}

/// `git check-ignore` works for paths that do not exist yet.
fn git_ignored(path: &str) -> bool {
    std::process::Command::new("git")
        .args(["check-ignore", "-q", "--", path])
        .current_dir(repo::root())
        .status()
        .is_ok_and(|s| s.success())
}

#[test]
fn generated_schema_counts_as_ignored() {
    assert!(git_ignored("fe/src/api/schema.d.ts"), "generated TS schema is gitignored");
    assert!(!git_ignored("crates/api/src/lib.rs"), "tracked source is not ignored");
}

#[test]
fn real_skills_dir_has_files() {
    let docs = skill_and_agent_docs();
    assert!(docs.len() >= 10, "expected ≥ 10 skill/agent docs, found {}", docs.len());
}

#[test]
fn skill_and_agent_refs_exist() {
    let violations = check_refs(&repo::root(), &skill_and_agent_docs(), &git_ignored);
    assert!(violations.is_empty(), "stale paths in skills/agents:\n{}", report(&violations));
}
