//! Every GitHub Actions job declares a bounded `timeout-minutes`.

use yaml_rust2::{Yaml, YamlLoader};

use crate::violation::Violation;

/// Upper bound for any job's `timeout-minutes`.
pub const MAX_TIMEOUT_MINUTES: u32 = 60;

fn key<'a>(map: &'a Yaml, name: &str) -> Option<&'a Yaml> {
    map.as_hash()?.get(&Yaml::String(name.to_owned()))
}

/// Violations for jobs in one workflow file. Reusable-workflow jobs
/// (`uses:`) cannot take `timeout-minutes` and are skipped; a file without
/// jobs, or that does not parse, is itself a violation.
#[must_use]
pub fn check_timeouts(path: &str, yaml: &str, max_minutes: u32) -> Vec<Violation> {
    let docs = match YamlLoader::load_from_str(yaml) {
        Ok(docs) => docs,
        Err(e) => return vec![Violation::new(path, format!("invalid YAML: {e}"))],
    };
    let Some(jobs) =
        docs.first().and_then(|d| key(d, "jobs")).and_then(Yaml::as_hash).filter(|m| !m.is_empty())
    else {
        return vec![Violation::new(path, "no jobs found")];
    };
    jobs.iter()
        .filter(|(_, job)| key(job, "uses").is_none())
        .filter_map(|(name, job)| {
            let name = name.as_str().unwrap_or("?");
            let detail = match key(job, "timeout-minutes") {
                None => format!("job `{name}` has no timeout-minutes"),
                Some(Yaml::Integer(m)) if (1..=i64::from(max_minutes)).contains(m) => return None,
                Some(other) => format!(
                    "job `{name}`: timeout-minutes must be an integer in 1..={max_minutes}, got {other:?}"
                ),
            };
            Some(Violation::new(path, detail))
        })
        .collect()
}

/// Path globs of one `dorny/paths-filter` group (`group`) in the workflow's
/// `changes` job, with YAML anchors (`*infra`) flattened in. Empty when the
/// job, the filter step or the group is missing.
#[must_use]
pub fn path_filter(yaml: &str, group: &str) -> Vec<String> {
    fn flatten(node: &Yaml, out: &mut Vec<String>) {
        match node {
            Yaml::String(s) => out.push(s.clone()),
            Yaml::Array(items) => items.iter().for_each(|i| flatten(i, out)),
            _ => {}
        }
    }
    let docs = YamlLoader::load_from_str(yaml).unwrap_or_default();
    let steps = docs
        .first()
        .and_then(|d| key(d, "jobs"))
        .and_then(|j| key(j, "changes"))
        .and_then(|c| key(c, "steps"))
        .and_then(Yaml::as_vec);
    let mut out = Vec::new();
    for filters in
        steps.into_iter().flatten().filter_map(|s| key(s, "with")).filter_map(|w| key(w, "filters"))
    {
        let parsed = filters.as_str().map(YamlLoader::load_from_str).and_then(Result::ok);
        if let Some(node) = parsed.as_ref().and_then(|d| d.first()).and_then(|d| key(d, group)) {
            flatten(node, &mut out);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILTERED: &str = "jobs:\n  changes:\n    timeout-minutes: 5\n    steps:\n      - uses: actions/checkout@v7\n      - uses: dorny/paths-filter@v4\n        with:\n          filters: |\n            infra: &infra\n              - 'compose.yaml'\n            backend:\n              - *infra\n              - 'crates/**'\n";

    #[test]
    fn path_filter_flattens_anchors() {
        assert_eq!(
            path_filter(FILTERED, "backend"),
            vec!["compose.yaml", "crates/**"],
            "anchor + own glob"
        );
    }

    #[test]
    fn path_filter_missing_group_is_empty() {
        assert!(path_filter(FILTERED, "frontend").is_empty(), "no such group");
        assert!(path_filter("jobs: {}\n", "backend").is_empty(), "no changes job");
        assert!(path_filter("jobs: [\n", "backend").is_empty(), "invalid yaml");
    }

    fn check(yaml: &str) -> Vec<Violation> {
        check_timeouts("w.yml", yaml, MAX_TIMEOUT_MINUTES)
    }

    #[test]
    fn bounded_timeout_passes() {
        assert!(
            check("jobs:\n  a:\n    timeout-minutes: 20\n    runs-on: x\n").is_empty(),
            "20 ≤ 60"
        );
    }

    #[test]
    fn missing_timeout_is_violation() {
        let v = check("jobs:\n  build:\n    runs-on: x\n");
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(v.iter().all(|x| x.detail.contains("build")), "names the job: {v:?}");
    }

    #[test]
    fn timeout_over_max_is_violation() {
        assert_eq!(check("jobs:\n  a:\n    timeout-minutes: 90\n").len(), 1, "90 > 60");
    }

    #[test]
    fn zero_timeout_is_violation() {
        assert_eq!(check("jobs:\n  a:\n    timeout-minutes: 0\n").len(), 1, "0");
    }

    #[test]
    fn negative_timeout_is_violation() {
        assert_eq!(check("jobs:\n  a:\n    timeout-minutes: -5\n").len(), 1, "-5");
    }

    #[test]
    fn expression_timeout_is_violation() {
        assert_eq!(
            check("jobs:\n  a:\n    timeout-minutes: ${{ inputs.t }}\n").len(),
            1,
            "expression"
        );
    }

    #[test]
    fn skips_reusable_workflow_jobs() {
        assert!(
            check("jobs:\n  publish:\n    uses: ./.github/workflows/x.yml\n").is_empty(),
            "uses: job"
        );
    }

    #[test]
    fn rejects_workflow_without_jobs() {
        assert_eq!(check("on: push\n").len(), 1, "no jobs key");
    }

    #[test]
    fn rejects_empty_jobs_map() {
        assert_eq!(check("jobs: {}\n").len(), 1, "empty jobs");
    }

    #[test]
    fn invalid_yaml_is_violation() {
        assert_eq!(check("jobs: [\n").len(), 1, "parse error");
    }
}
