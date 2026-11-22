//! Every helper script in `scripts/` and every `rdt` command in
//! `.rusty_dev_tool/config.toml` is documented in the README.

use crate::violation::Violation;

/// One `[commands.<name>]` section of the rdt config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdtCommand {
    pub name: String,
    pub alias: Option<String>,
}

/// Contents of every inline code span (`` `…` ``) in `markdown`.
#[must_use]
pub fn code_spans(markdown: &str) -> Vec<&str> {
    markdown.split('`').skip(1).step_by(2).collect()
}

/// `[commands.<name>]` sections and their `"alias"`, in file order.
#[must_use]
pub fn rdt_commands(config: &str) -> Vec<RdtCommand> {
    let mut out: Vec<RdtCommand> = Vec::new();
    for line in config.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix("[commands.").and_then(|r| r.strip_suffix(']')) {
            out.push(RdtCommand { name: name.to_owned(), alias: None });
        } else if let Some(value) =
            line.strip_prefix("\"alias\"").and_then(|r| r.trim_start().strip_prefix('='))
            && let Some(last) = out.last_mut()
        {
            last.alias = Some(value.trim().trim_matches('"').to_owned());
        }
    }
    out
}

/// Each script must appear as `scripts/<file>` inside a README code span.
#[must_use]
pub fn check_scripts_documented(scripts: &[String], readme: &str) -> Vec<Violation> {
    if scripts.is_empty() {
        return vec![Violation::new("scripts/", "no scripts found (wrong working directory?)")];
    }
    let spans = code_spans(readme);
    scripts
        .iter()
        .filter(|file| {
            let needle = format!("scripts/{file}");
            !spans
                .iter()
                .any(|span| span.split_whitespace().any(|w| w.trim_start_matches("./") == needle))
        })
        .map(|file| {
            Violation::new(
                "README.md",
                format!("`scripts/{file}` is not documented in the scripts table"),
            )
        })
        .collect()
}

/// Each rdt command must appear as `` `rdt <name-or-alias>[ …]` `` in the README.
#[must_use]
pub fn check_rdt_documented(commands: &[RdtCommand], readme: &str) -> Vec<Violation> {
    if commands.is_empty() {
        return vec![Violation::new(".rusty_dev_tool/config.toml", "no rdt commands found")];
    }
    let documented: Vec<&str> = code_spans(readme)
        .into_iter()
        .filter_map(|span| span.strip_prefix("rdt "))
        .filter_map(|rest| rest.split_whitespace().next())
        .collect();
    commands
        .iter()
        .filter(|c| !documented.iter().any(|d| *d == c.name || c.alias.as_deref() == Some(*d)))
        .map(|c| {
            Violation::new(
                "README.md",
                format!("`rdt {}` is not documented in the commands table", c.name),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const README: &str = "| `./scripts/a.sh check` | does a |\n| `rdt migrate` | m |\n| `rdt l` | lint |\nplain scripts/b.sh mention\n";

    #[test]
    fn code_spans_are_extracted() {
        assert_eq!(code_spans("x `a b` y `c`"), vec!["a b", "c"], "backtick spans");
    }

    #[test]
    fn documented_script_passes() {
        assert!(
            check_scripts_documented(&["a.sh".to_owned()], README).is_empty(),
            "a.sh is in a code span"
        );
    }

    #[test]
    fn undocumented_script_is_violation() {
        let v = check_scripts_documented(&["c.sh".to_owned()], README);
        assert_eq!(v.len(), 1, "c.sh missing: {v:?}");
        assert!(v.iter().all(|x| x.detail.contains("scripts/c.sh")), "names the script");
    }

    #[test]
    fn prose_mention_does_not_count() {
        assert_eq!(
            check_scripts_documented(&["b.sh".to_owned()], README).len(),
            1,
            "b.sh only in prose"
        );
    }

    #[test]
    fn parses_rdt_commands_with_alias() {
        let toml = "[commands.migrate]\n  \"alias\" = \"m\"\n[commands.lint]\n  \"description\" = \"x\"\n  \"alias\" = \"l\"\n[commands.fe]\n  \"execution\" = \"y\"\n";
        assert_eq!(
            rdt_commands(toml),
            vec![
                RdtCommand { name: "migrate".to_owned(), alias: Some("m".to_owned()) },
                RdtCommand { name: "lint".to_owned(), alias: Some("l".to_owned()) },
                RdtCommand { name: "fe".to_owned(), alias: None },
            ],
            "name + optional alias per section"
        );
    }

    #[test]
    fn rdt_command_by_name_or_alias_passes() {
        let cmds = rdt_commands("[commands.migrate]\n[commands.lint]\n\"alias\" = \"l\"\n");
        assert!(check_rdt_documented(&cmds, README).is_empty(), "migrate by name, lint by alias");
    }

    #[test]
    fn rdt_prefix_is_not_a_match() {
        let cmds = rdt_commands("[commands.migrate-check]\n\"alias\" = \"migrate-check\"\n");
        assert_eq!(
            check_rdt_documented(&cmds, README).len(),
            1,
            "`rdt migrate` does not document migrate-check"
        );
    }

    #[test]
    fn empty_inputs_are_violations() {
        assert_eq!(check_scripts_documented(&[], README).len(), 1, "no scripts found");
        assert_eq!(check_rdt_documented(&[], README).len(), 1, "no rdt commands found");
    }
}
