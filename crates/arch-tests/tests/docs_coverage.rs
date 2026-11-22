//! The README documents every helper script and every rdt command.

use std::fs;

use my_fam_tree_arch_tests::docs::{check_rdt_documented, check_scripts_documented, rdt_commands};
use my_fam_tree_arch_tests::repo;
use my_fam_tree_arch_tests::violation::report;

fn readme() -> String {
    fs::read_to_string(repo::root().join("README.md")).unwrap_or_default()
}

#[test]
fn every_script_is_documented() {
    let mut scripts: Vec<String> = fs::read_dir(repo::root().join("scripts"))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    scripts.sort();
    let violations = check_scripts_documented(&scripts, &readme());
    assert!(violations.is_empty(), "undocumented scripts:\n{}", report(&violations));
}

#[test]
fn every_rdt_command_is_documented() {
    let config =
        fs::read_to_string(repo::root().join(".rusty_dev_tool/config.toml")).unwrap_or_default();
    let violations = check_rdt_documented(&rdt_commands(&config), &readme());
    assert!(violations.is_empty(), "undocumented rdt commands:\n{}", report(&violations));
}
