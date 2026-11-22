//! Scripts shipped with skills run and produce what the skill promises.

use std::process::Command;

use my_fam_tree_arch_tests::repo;

#[test]
fn routes_script_lists_health() {
    if Command::new("jq").arg("--version").output().is_err() {
        eprintln!("skipping routes_script_lists_health: jq not installed");
        return;
    }
    let out = Command::new("bash")
        .arg(repo::root().join(".claude/skills/crate-api/scripts/routes.sh"))
        .output();
    assert!(out.is_ok(), "bash must start: {out:?}");
    let out = out.unwrap_or_else(|_| unreachable!());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "routes.sh failed: {}", String::from_utf8_lossy(&out.stderr));
    assert!(
        stdout.lines().any(|l| l.starts_with("GET /api/v1/health")),
        "health route listed:\n{stdout}"
    );
    assert!(stdout.lines().count() >= 20, "expected the full route table, got:\n{stdout}");
}
