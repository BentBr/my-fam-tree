//! `/qa` scope script: which files a QA run covers and how they are classed.
//! Runs `.claude/skills/qa/scripts/scope.sh` against throwaway git repos.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use my_fam_tree_arch_tests::repo;

/// Variables through which an outer git process (e.g. a pre-push hook run
/// from a linked worktree, which exports `GIT_DIR`) would redirect these
/// commands to the real repository.
const INHERITED_GIT_VARS: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_PREFIX",
    "GIT_CONFIG_PARAMETERS",
    "GIT_COMMON_DIR",
];

/// A command that only ever sees the throwaway repo in its cwd: no inherited
/// repo variables, no user/system git config (hooks path, gpg signing).
fn isolated(mut cmd: Command) -> Command {
    for var in INHERITED_GIT_VARS {
        cmd.env_remove(var);
    }
    cmd.env("GIT_CONFIG_GLOBAL", "/dev/null").env("GIT_CONFIG_NOSYSTEM", "1");
    cmd
}

struct Repo(PathBuf);

impl Repo {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("qa-scope-{name}-{}", std::process::id()));
        drop(fs::remove_dir_all(&dir));
        assert!(fs::create_dir_all(&dir).is_ok(), "mkdir");
        let r = Self(dir);
        r.git(&["init", "-q", "-b", "main"]);
        r.git(&["config", "user.email", "qa@example.com"]);
        r.git(&["config", "user.name", "qa"]);
        r.write("README.md", "x\n");
        r.commit("init");
        r
    }

    fn git(&self, args: &[&str]) {
        let st = isolated(Command::new("git")).args(args).current_dir(&self.0).status();
        assert!(st.is_ok_and(|s| s.success()), "git {args:?}");
    }

    fn write(&self, rel: &str, content: &str) {
        let p = self.0.join(rel);
        if let Some(parent) = p.parent() {
            assert!(fs::create_dir_all(parent).is_ok(), "mkdir parent");
        }
        assert!(fs::write(&p, content).is_ok(), "write {rel}");
    }

    fn commit(&self, msg: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", msg]);
    }

    fn scope(&self, mode: Option<&str>) -> String {
        let mut cmd = isolated(Command::new("bash"));
        cmd.arg(repo::root().join(".claude/skills/qa/scripts/scope.sh"));
        if let Some(m) = mode {
            cmd.arg(m);
        }
        let out =
            cmd.current_dir(&self.0).env("QA_BASE_REF", "main").env("QA_NO_FETCH", "1").output();
        assert!(out.is_ok(), "bash must start");
        let out = out.unwrap_or_else(|_| unreachable!());
        assert!(out.status.success(), "scope.sh failed: {}", String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

#[test]
fn git_commands_ignore_inherited_repo_env() {
    let cmd = isolated(Command::new("git"));
    let envs: Vec<_> = cmd.get_envs().collect();
    for var in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_PREFIX", "GIT_CONFIG_PARAMETERS"]
    {
        assert!(
            envs.iter().any(|(k, v)| *k == var && v.is_none()),
            "{var} must be removed: {envs:?}"
        );
    }
    assert!(
        envs.iter().any(|(k, v)| *k == "GIT_CONFIG_NOSYSTEM" && v.is_some()),
        "system config disabled: {envs:?}"
    );
    assert!(
        envs.iter().any(|(k, v)| *k == "GIT_CONFIG_GLOBAL" && v.is_some()),
        "global config isolated: {envs:?}"
    );
}

fn has(out: &str, class: &str, file: &str) -> bool {
    out.lines().any(|l| l == format!("{class}\t{file}"))
}

#[test]
fn whole_branch_covers_commits_and_local_edits() {
    let r = Repo::new("branch");
    r.git(&["checkout", "-q", "-b", "feat/x"]);
    r.write("crates/api/src/a.rs", "fn a() {}\n");
    r.commit("feat: a");
    r.write("clippy.toml", "x = 1\n");
    r.write("fe/src/b.ts", "export {};\n");
    let out = r.scope(None);
    assert!(out.lines().next().is_some_and(|l| l.starts_with("# mode: branch")), "header: {out}");
    assert!(has(&out, "rust-local:api", "crates/api/src/a.rs"), "committed crate file: {out}");
    assert!(has(&out, "rust-shared", "clippy.toml"), "untracked shared rust file: {out}");
    assert!(has(&out, "fe-local", "fe/src/b.ts"), "untracked fe file: {out}");
    assert!(r.path().exists(), "repo exists");
}

#[test]
fn whole_branch_still_covers_commits_after_push() {
    let r = Repo::new("pushed");
    r.git(&["checkout", "-q", "-b", "feat/y"]);
    r.write("crates/domain/src/d.rs", "\n");
    r.commit("feat: d");
    r.git(&["branch", "-q", "pushed"]);
    r.git(&["branch", "-q", "--set-upstream-to=pushed"]);
    let out = r.scope(None);
    assert!(
        has(&out, "rust-local:domain", "crates/domain/src/d.rs"),
        "branch scope after push: {out}"
    );
}

#[test]
fn delta_with_nothing_new_says_so() {
    let r = Repo::new("delta-empty");
    r.git(&["checkout", "-q", "-b", "feat/z"]);
    r.write("crates/api/src/a.rs", "\n");
    r.commit("feat: a");
    r.git(&["branch", "-q", "pushed"]);
    r.git(&["branch", "-q", "--set-upstream-to=pushed"]);
    let out = r.scope(Some("delta"));
    assert!(out.contains("nothing new since upstream"), "explicit empty delta: {out}");
    assert!(!out.contains("crates/api/src/a.rs"), "pushed commit not in delta: {out}");
}

#[test]
fn delta_includes_uncommitted_edits() {
    let r = Repo::new("delta-local");
    r.git(&["checkout", "-q", "-b", "feat/w"]);
    r.git(&["branch", "-q", "pushed"]);
    r.git(&["branch", "-q", "--set-upstream-to=pushed"]);
    r.write("README.md", "changed\n");
    r.write("fe/eslint.config.js", "export default [];\n");
    let out = r.scope(Some("delta"));
    assert!(has(&out, "docs", "README.md"), "modified tracked file: {out}");
    assert!(has(&out, "fe-shared", "fe/eslint.config.js"), "shared FE config: {out}");
}

#[test]
fn classifies_ci_and_skill_files() {
    let r = Repo::new("classes");
    r.git(&["checkout", "-q", "-b", "ci/x"]);
    r.write(".github/workflows/ci.yml", "jobs: {}\n");
    r.write(".claude/skills/qa/SKILL.md", "x\n");
    r.write("migrations/0001_x.sql", "\n");
    r.write("fe/package.json", "{}\n");
    let out = r.scope(None);
    assert!(has(&out, "ci", ".github/workflows/ci.yml"), "{out}");
    assert!(has(&out, "docs", ".claude/skills/qa/SKILL.md"), "{out}");
    assert!(has(&out, "rust-shared", "migrations/0001_x.sql"), "{out}");
    assert!(has(&out, "fe-shared", "fe/package.json"), "{out}");
}
