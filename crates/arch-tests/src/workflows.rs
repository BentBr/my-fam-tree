//! Every GitHub Actions job declares a bounded `timeout-minutes`.

use crate::violation::Violation;

/// Upper bound for any job's `timeout-minutes`.
pub const MAX_TIMEOUT_MINUTES: u32 = 60;

use serde_yaml_ng::Value;

/// Violations for jobs in one workflow file. Reusable-workflow jobs
/// (`uses:`) cannot take `timeout-minutes` and are skipped; a file without
/// jobs, or that does not parse, is itself a violation.
#[must_use]
pub fn check_timeouts(path: &str, yaml: &str, max_minutes: u32) -> Vec<Violation> {
    let doc: Value = match serde_yaml_ng::from_str(yaml) {
        Ok(doc) => doc,
        Err(e) => return vec![Violation::new(path, format!("invalid YAML: {e}"))],
    };
    let Some(jobs) = doc.get("jobs").and_then(Value::as_mapping).filter(|m| !m.is_empty()) else {
        return vec![Violation::new(path, "no jobs found")];
    };
    jobs.iter()
        .filter(|(_, job)| job.get("uses").is_none())
        .filter_map(|(name, job)| {
            let name = name.as_str().unwrap_or("?");
            let detail = match job.get("timeout-minutes") {
                None => format!("job `{name}` has no timeout-minutes"),
                Some(Value::Number(n))
                    if n.as_u64().is_some_and(|m| m >= 1 && m <= u64::from(max_minutes)) =>
                {
                    return None;
                }
                Some(other) => format!(
                    "job `{name}`: timeout-minutes must be an integer in 1..={max_minutes}, got {}",
                    serde_yaml_ng::to_string(other).unwrap_or_default().trim()
                ),
            };
            Some(Violation::new(path, detail))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
