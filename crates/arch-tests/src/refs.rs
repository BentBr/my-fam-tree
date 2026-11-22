//! Every `cargo … -p <pkg>` in hooks, rdt config, scripts and workflows must
//! name a real workspace package.

use crate::violation::Violation;

/// Programs whose `-p` / `--package` arguments name a cargo package.
const CARGO_PROGRAMS: &[&str] = &["cargo", "cargo-in-network.sh"];

/// Package names passed to cargo via `-p`, `--package` or `--package=` in
/// `text`.
///
/// Each line is split into simple commands on `&&`, `||`, `;` and `|`.
/// Within a command the scan starts at the first word that is a cargo
/// program (so env assignments, `+toolchain`, and wrappers such as
/// `bash -c "cargo …"` are handled) and stops at a bare `--`.
#[must_use]
pub fn cargo_package_refs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        for command in split_commands(line) {
            collect_from_command(&command, &mut out);
        }
    }
    out
}

fn split_commands(line: &str) -> Vec<String> {
    let mut parts = vec![String::new()];
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ';' | '|' | '&' => {
                if matches!(c, '|' | '&') && chars.peek() == Some(&c) {
                    chars.next();
                }
                parts.push(String::new());
            }
            '"' | '\'' => {}
            _ => {
                if let Some(last) = parts.last_mut() {
                    last.push(c);
                }
            }
        }
    }
    parts
}

fn is_cargo_program(word: &str) -> bool {
    let base = word.rsplit('/').next().unwrap_or(word);
    CARGO_PROGRAMS.contains(&base)
}

fn collect_from_command(command: &str, out: &mut Vec<String>) {
    let mut words = command.split_whitespace().skip_while(|w| !is_cargo_program(w));
    if words.next().is_none() {
        return;
    }
    while let Some(word) = words.next() {
        if word == "--" {
            break;
        }
        if let Some(name) = word.strip_prefix("--package=") {
            out.push(name.to_owned());
        } else if matches!(word, "-p" | "--package")
            && let Some(name) = words.next()
        {
            out.push(name.to_owned());
        }
    }
}

/// Violations for every referenced package that is not in `packages`.
/// `files` are `(path, contents)` pairs.
#[must_use]
pub fn check_package_refs(files: &[(String, String)], packages: &[String]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|(path, text)| {
            cargo_package_refs(text).into_iter().filter(|name| !packages.contains(name)).map(
                move |name| Violation::new(path.clone(), format!("unknown cargo package `{name}`")),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_dash_p() {
        assert_eq!(
            cargo_package_refs("cargo run -p my-fam-tree-openapi --bin x"),
            vec!["my-fam-tree-openapi"],
            "short -p form"
        );
    }

    #[test]
    fn extracts_long_and_equals_forms() {
        assert_eq!(
            cargo_package_refs("cargo test --package a && cargo build --package=b"),
            vec!["a", "b"],
            "--package and --package= forms"
        );
    }

    #[test]
    fn ignores_non_cargo_dash_p() {
        assert!(
            cargo_package_refs("mkdir -p target/x\ndocker compose -p proj up\ncp -p a b")
                .is_empty(),
            "-p of other tools is not a package"
        );
    }

    #[test]
    fn same_line_mkdir_and_cargo() {
        assert_eq!(
            cargo_package_refs("mkdir -p target && cargo build -p a"),
            vec!["a"],
            "only the cargo command counts"
        );
    }

    #[test]
    fn stops_at_double_dash() {
        assert_eq!(
            cargo_package_refs("cargo run -p a --bin x -- -p b"),
            vec!["a"],
            "args after -- belong to the binary"
        );
    }

    #[test]
    fn toolchain_and_env_prefix() {
        assert_eq!(
            cargo_package_refs("SQLX_OFFLINE=true cargo +nightly clippy -p a"),
            vec!["a"],
            "env assignment and +toolchain are skipped"
        );
    }

    #[test]
    fn wrapped_in_bash_c() {
        assert_eq!(
            cargo_package_refs(r#"run "x" SKIP bash -c "cargo run -p a --bin d | diff - f.json""#),
            vec!["a"],
            "cargo inside a quoted bash -c"
        );
    }

    #[test]
    fn network_wrapper_counts() {
        assert_eq!(
            cargo_package_refs("./scripts/cargo-in-network.sh test -p a --test t"),
            vec!["a"],
            "cargo-in-network.sh forwards to cargo"
        );
    }

    #[test]
    fn flags_unknown_package() {
        let files =
            vec![(".githooks/pre-push".to_owned(), "cargo run -p my-family-openapi".to_owned())];
        let v = check_package_refs(&files, &["my-fam-tree-openapi".to_owned()]);
        assert_eq!(v.len(), 1, "one unknown package");
        assert!(
            v.iter().all(|x| x.detail.contains("my-family-openapi")),
            "names the bad package: {v:?}"
        );
    }

    #[test]
    fn known_package_passes() {
        let files = vec![("x".to_owned(), "cargo run -p a".to_owned())];
        assert!(check_package_refs(&files, &["a".to_owned()]).is_empty(), "known package");
    }
}
