//! Every `cargo -p <pkg>` in hooks, rdt config, scripts and workflows names a
//! real workspace package.

use std::fs;
use std::path::Path;

use my_fam_tree_arch_tests::refs::check_package_refs;
use my_fam_tree_arch_tests::repo;
use my_fam_tree_arch_tests::violation::report;

fn files_in(root: &Path, rel: &str, keep: fn(&Path) -> bool) -> Vec<(String, String)> {
    let Ok(entries) = fs::read_dir(root.join(rel)) else { return Vec::new() };
    let mut out: Vec<(String, String)> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && keep(p))
        .filter_map(|p| {
            let name = p.strip_prefix(root).unwrap_or(&p).display().to_string();
            fs::read_to_string(&p).ok().map(|text| (name, text))
        })
        .collect();
    out.sort();
    out
}

fn has_ext(p: &Path, exts: &[&str]) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| exts.contains(&e))
}

#[test]
fn all_cargo_package_refs_exist() {
    let root = repo::root();
    let mut files = files_in(&root, ".githooks", |_| true);
    files.extend(files_in(&root, ".rusty_dev_tool", |p| has_ext(p, &["toml"])));
    files.extend(files_in(&root, "scripts", |p| has_ext(p, &["sh"])));
    files.extend(files_in(&root, ".github/workflows", |p| has_ext(p, &["yml", "yaml"])));
    assert!(
        files.len() >= 5,
        "expected hook, rdt, script and workflow files, found {}",
        files.len()
    );

    let packages = repo::workspace_packages().unwrap_or_default();
    assert!(packages.len() >= 12, "expected ≥ 12 workspace packages, found {packages:?}");

    let violations = check_package_refs(&files, &packages);
    assert!(violations.is_empty(), "unknown cargo packages:\n{}", report(&violations));
}
