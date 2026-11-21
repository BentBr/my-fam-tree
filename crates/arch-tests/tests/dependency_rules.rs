//! The real workspace respects the crate dependency matrix in `rules.rs`.

use my_fam_tree_arch_tests::deps::{check, from_metadata};
use my_fam_tree_arch_tests::repo;
use my_fam_tree_arch_tests::rules::RULES;
use my_fam_tree_arch_tests::violation::report;

#[test]
fn workspace_respects_dependency_rules() {
    let meta = repo::metadata();
    assert!(meta.is_ok(), "cargo metadata failed: {meta:?}");
    let crates = meta.as_ref().map_or_else(|_| Vec::new(), from_metadata);
    let violations = check(&crates, RULES);
    assert!(violations.is_empty(), "dependency rule violations:\n{}", report(&violations));
}
