//! `scripts/openapi.sh` reports generator failures honestly and never
//! clobbers the committed spec when generation fails.
//!
//! A stub generator stands in for `cargo run … openapi-dump` via
//! `OPENAPI_DUMP_CMD`, so these tests need no cargo build.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

use my_fam_tree_arch_tests::repo;

struct Sandbox {
    dir: PathBuf,
    spec: PathBuf,
}

impl Sandbox {
    fn new(name: &str, spec_content: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("openapi-script-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        assert!(fs::create_dir_all(&dir).is_ok(), "create sandbox dir");
        let spec = dir.join("openapi.json");
        assert!(fs::write(&spec, spec_content).is_ok(), "write spec");
        Self { dir, spec }
    }

    fn run(&self, mode: &str, dump_cmd: &str) -> ExitStatus {
        let status = Command::new("bash")
            .arg(repo::root().join("scripts/openapi.sh"))
            .arg(mode)
            .env("OPENAPI_SPEC", &self.spec)
            .env("OPENAPI_DUMP_CMD", dump_cmd)
            .current_dir(&self.dir)
            .status();
        assert!(status.is_ok(), "bash must start: {status:?}");
        status.unwrap_or_else(|_| unreachable!())
    }

    fn spec(&self) -> String {
        fs::read_to_string(&self.spec).unwrap_or_default()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn check_matching_spec_exits_0() {
    let sb = Sandbox::new("match", "{\"a\":1}\n");
    assert!(sb.run("check", "printf '{\"a\":1}\\n'").success(), "identical output is no drift");
}

#[test]
fn check_stale_spec_exits_nonzero() {
    let sb = Sandbox::new("stale", "{\"a\":1}\n");
    assert!(!sb.run("check", "printf '{\"a\":2}\\n'").success(), "different output is drift");
}

#[test]
fn check_generator_failure_exits_nonzero() {
    let sb = Sandbox::new("genfail", "{\"a\":1}\n");
    let status = sb.run("check", "sh -c 'cat \"$OPENAPI_SPEC\"; exit 3'");
    assert_eq!(
        status.code(),
        Some(3),
        "matching output then failure must surface the generator's status"
    );
}

#[test]
fn write_generator_failure_keeps_spec() {
    let sb = Sandbox::new("writefail", "{\"keep\":true}\n");
    assert!(
        !sb.run("write", "sh -c 'printf partial; exit 2'").success(),
        "failure must be reported"
    );
    assert_eq!(sb.spec(), "{\"keep\":true}\n", "spec must be untouched");
}

#[test]
fn write_success_replaces_spec() {
    let sb = Sandbox::new("writeok", "{\"old\":true}\n");
    assert!(sb.run("write", "printf '{\"new\":true}\\n'").success(), "write succeeds");
    assert_eq!(sb.spec(), "{\"new\":true}\n", "spec replaced");
}

#[test]
fn unknown_mode_exits_nonzero() {
    let sb = Sandbox::new("mode", "{}\n");
    assert!(!sb.run("bogus", "printf '{}\\n'").success(), "usage error");
}

#[cfg(unix)]
#[test]
fn write_keeps_spec_file_mode() {
    use std::os::unix::fs::PermissionsExt;
    let sb = Sandbox::new("mode-keep", "{}\n");
    assert!(fs::set_permissions(&sb.spec, fs::Permissions::from_mode(0o644)).is_ok(), "chmod");
    assert!(sb.run("write", "printf '{\"n\":1}\\n'").success(), "write succeeds");
    let mode = fs::metadata(&sb.spec).map_or(0, |m| m.permissions().mode() & 0o777);
    assert_eq!(mode, 0o644, "spec keeps its file mode (mktemp files are 0600)");
}
