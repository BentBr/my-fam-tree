//! Exponential backoff with jitter for digest send retries.

use chrono::{DateTime, Duration as CDur, Utc};
use rand::Rng;

/// Next retry instant for `attempt` (1-based): `min * 2^(attempt-1)` capped at
/// `max`, plus up to 25% jitter so a fleet of retries doesn't thunder.
#[must_use]
pub fn next_attempt(now: DateTime<Utc>, attempt: i32, min_sec: u64, max_sec: u64) -> DateTime<Utc> {
    let exp = u32::try_from(attempt.max(1) - 1).unwrap_or(0);
    let base = min_sec.saturating_mul(2_u64.saturating_pow(exp)).min(max_sec);
    let jitter = rand::thread_rng().gen_range(0..=(base / 4 + 1));
    let secs = i64::try_from(base.saturating_add(jitter)).unwrap_or(i64::MAX);
    now + CDur::seconds(secs)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).single().unwrap()
    }

    #[test]
    fn grows_monotonically_then_caps() {
        let n = now();
        let d1 = (next_attempt(n, 1, 60, 43_200) - n).num_seconds();
        let d4 = (next_attempt(n, 4, 60, 43_200) - n).num_seconds();
        // attempt 1 ~ 60s (+jitter), attempt 4 ~ 480s (+jitter) — strictly larger.
        assert!((60..=60 + 60 / 4 + 1).contains(&d1));
        assert!(d4 >= 480);
    }

    #[test]
    fn never_exceeds_max_plus_jitter() {
        let n = now();
        let big = next_attempt(n, 30, 60, 43_200);
        let secs = (big - n).num_seconds();
        assert!(secs <= 43_200 + 43_200 / 4 + 1, "capped at max + 25% jitter, got {secs}");
    }

    #[test]
    fn attempt_zero_or_negative_treated_as_first() {
        let n = now();
        let d = (next_attempt(n, 0, 60, 43_200) - n).num_seconds();
        assert!((60..=60 + 60 / 4 + 1).contains(&d), "attempt 0 behaves like attempt 1");
    }
}
