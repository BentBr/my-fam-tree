//! Crate dependency matrix: which workspace crates may depend on which
//! internal crates and which external dependency groups.

use crate::rules::Rule;
use crate::violation::Violation;

/// One normal (non-dev, non-build) dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepRef {
    pub name: String,
    pub optional: bool,
}

/// A workspace crate and its normal dependencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateDeps {
    pub name: String,
    pub normal: Vec<DepRef>,
}

const INTERNAL_PREFIX: &str = "my-fam-tree-";

/// Workspace members with their normal dependencies.
#[must_use]
pub fn from_metadata(meta: &cargo_metadata::Metadata) -> Vec<CrateDeps> {
    meta.workspace_packages()
        .iter()
        .map(|pkg| CrateDeps {
            name: pkg.name.to_string(),
            normal: pkg
                .dependencies
                .iter()
                .filter(|d| d.kind == cargo_metadata::DependencyKind::Normal)
                .map(|d| DepRef { name: d.name.clone(), optional: d.optional })
                .collect(),
        })
        .collect()
}

/// Violations of `rules` by `crates`.
#[must_use]
pub fn check(crates: &[CrateDeps], rules: &[Rule]) -> Vec<Violation> {
    if crates.is_empty() {
        return vec![Violation::new(
            "workspace",
            "no workspace crates found (wrong working directory?)",
        )];
    }
    let mut out = Vec::new();
    for krate in crates {
        let Some(rule) = rules.iter().find(|r| r.krate == krate.name) else {
            out.push(Violation::new(
                &krate.name,
                "no dependency rule (add one to arch-tests rules.rs)",
            ));
            continue;
        };
        out.extend(krate.normal.iter().filter_map(|dep| dep_violation(rule, dep)));
    }
    out.extend(
        rules
            .iter()
            .filter(|r| !crates.iter().any(|c| c.name == r.krate))
            .map(|r| Violation::new(r.krate, "stale rule: crate is not in the workspace")),
    );
    out
}

fn dep_violation(rule: &Rule, dep: &DepRef) -> Option<Violation> {
    let name = dep.name.as_str();
    if name.starts_with(INTERNAL_PREFIX) {
        return (!rule.allowed_internal.contains(&name)).then(|| {
            Violation::new(rule.krate, format!("must not depend on internal crate `{name}`"))
        });
    }
    let forbidden = rule.forbidden_external.contains(&name);
    let optional_ok = dep.optional && rule.optional_ok.contains(&name);
    let excepted = rule.exceptions.iter().any(|(d, _)| *d == name);
    (forbidden && !optional_ok && !excepted)
        .then(|| Violation::new(rule.krate, format!("must not depend on `{name}`")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULES: &[Rule] = &[
        Rule {
            krate: "my-fam-tree-domain",
            allowed_internal: &[],
            forbidden_external: &["sqlx", "utoipa"],
            optional_ok: &["utoipa"],
            exceptions: &[],
        },
        Rule {
            krate: "my-fam-tree-persistence",
            allowed_internal: &["my-fam-tree-domain"],
            forbidden_external: &["actix-web"],
            optional_ok: &[],
            exceptions: &[],
        },
        Rule {
            krate: "my-fam-tree-migrator",
            allowed_internal: &[],
            forbidden_external: &["sqlx"],
            optional_ok: &[],
            exceptions: &[("sqlx", "runs migrations")],
        },
    ];

    fn krate(name: &str, deps: &[(&str, bool)]) -> CrateDeps {
        CrateDeps {
            name: name.to_owned(),
            normal: deps
                .iter()
                .map(|(n, o)| DepRef { name: (*n).to_owned(), optional: *o })
                .collect(),
        }
    }

    fn all_three(domain: CrateDeps) -> Vec<CrateDeps> {
        vec![domain, krate("my-fam-tree-persistence", &[]), krate("my-fam-tree-migrator", &[])]
    }

    #[test]
    fn clean_workspace_passes() {
        assert!(
            check(&all_three(krate("my-fam-tree-domain", &[("serde", false)])), RULES).is_empty(),
            "no violations"
        );
    }

    #[test]
    fn forbidden_external_is_violation() {
        let v = check(&all_three(krate("my-fam-tree-domain", &[("sqlx", false)])), RULES);
        assert_eq!(v.len(), 1, "domain → sqlx: {v:?}");
        assert!(
            v.iter().all(|x| x.file == "my-fam-tree-domain" && x.detail.contains("sqlx")),
            "{v:?}"
        );
    }

    #[test]
    fn optional_ok_passes() {
        assert!(
            check(&all_three(krate("my-fam-tree-domain", &[("utoipa", true)])), RULES).is_empty(),
            "optional utoipa"
        );
    }

    #[test]
    fn optional_ok_requires_optional() {
        assert_eq!(
            check(&all_three(krate("my-fam-tree-domain", &[("utoipa", false)])), RULES).len(),
            1,
            "non-optional utoipa is a violation"
        );
    }

    #[test]
    fn disallowed_internal_is_violation() {
        let crates = vec![
            krate("my-fam-tree-domain", &[]),
            krate("my-fam-tree-persistence", &[("my-fam-tree-api", false)]),
            krate("my-fam-tree-migrator", &[]),
        ];
        let v = check(&crates, RULES);
        assert_eq!(v.len(), 1, "persistence → api: {v:?}");
        assert!(v.iter().all(|x| x.detail.contains("my-fam-tree-api")), "{v:?}");
    }

    #[test]
    fn exception_passes() {
        let crates = vec![
            krate("my-fam-tree-domain", &[]),
            krate("my-fam-tree-persistence", &[]),
            krate("my-fam-tree-migrator", &[("sqlx", false)]),
        ];
        assert!(check(&crates, RULES).is_empty(), "migrator sqlx exception");
    }

    #[test]
    fn crate_without_rule_is_violation() {
        let mut crates = all_three(krate("my-fam-tree-domain", &[]));
        crates.push(krate("my-fam-tree-new", &[]));
        let v = check(&crates, RULES);
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(
            v.iter()
                .all(|x| x.file == "my-fam-tree-new" && x.detail.contains("no dependency rule")),
            "{v:?}"
        );
    }

    #[test]
    fn stale_rule_is_violation() {
        let crates = vec![krate("my-fam-tree-domain", &[]), krate("my-fam-tree-persistence", &[])];
        let v = check(&crates, RULES);
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(
            v.iter().all(|x| x.file == "my-fam-tree-migrator" && x.detail.contains("stale rule")),
            "{v:?}"
        );
    }

    #[test]
    fn rejects_empty_workspace() {
        let v = check(&[], RULES);
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(v.iter().all(|x| x.detail.contains("no workspace crates found")), "{v:?}");
    }

    #[test]
    fn real_rules_cover_internal_prefix_only() {
        assert!(
            crate::rules::RULES.iter().all(|r| r.krate.starts_with("my-fam-tree-")),
            "rules name full package names"
        );
    }
}
