//! Every job in `.github/workflows` has a bounded `timeout-minutes`.

use std::fs;
use std::path::{Path, PathBuf};

use my_fam_tree_arch_tests::repo;
use my_fam_tree_arch_tests::violation::report;
use my_fam_tree_arch_tests::workflows::{MAX_TIMEOUT_MINUTES, check_timeouts};

fn workflow_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "yml" || e == "yaml"))
        .collect();
    files.sort();
    files
}

#[test]
fn collects_yml_and_yaml() {
    let dir = std::env::temp_dir().join(format!("wf-collect-{}", std::process::id()));
    drop(fs::remove_dir_all(&dir));
    assert!(fs::create_dir_all(&dir).is_ok(), "mkdir");
    for name in ["a.yml", "b.yaml", "c.txt"] {
        assert!(fs::write(dir.join(name), "jobs: {}\n").is_ok(), "write {name}");
    }
    let found = workflow_files(&dir);
    drop(fs::remove_dir_all(&dir));
    assert_eq!(found.len(), 2, "yml + yaml, not txt: {found:?}");
}

#[test]
fn all_workflow_jobs_have_timeouts() {
    let root = repo::root();
    let files = workflow_files(&root.join(".github/workflows"));
    assert!(files.len() >= 3, "expected ≥ 3 workflow files, found {files:?}");
    let violations: Vec<_> = files
        .iter()
        .flat_map(|p| {
            let name = p.strip_prefix(&root).unwrap_or(p).display().to_string();
            let yaml = fs::read_to_string(p).unwrap_or_default();
            check_timeouts(&name, &yaml, MAX_TIMEOUT_MINUTES)
        })
        .collect();
    assert!(violations.is_empty(), "workflow timeout violations:\n{}", report(&violations));
}
