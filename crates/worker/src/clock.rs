//! Clock abstraction so the worker's "now" is injectable in tests + E2E.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, TimeZone, Utc};

/// Source of "now". Production uses [`SystemClock`]; tests + the
/// `test-fixtures` HTTP listener use [`FixedClock`].
pub trait Clock: Send + Sync + std::fmt::Debug {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Lock-free settable clock. The instant is microseconds since the Unix epoch
/// in an atomic, so reads never block and writes never panic.
#[derive(Debug, Clone)]
pub struct FixedClock {
    micros: Arc<AtomicI64>,
}

impl FixedClock {
    #[must_use]
    pub fn new(at: DateTime<Utc>) -> Self {
        Self { micros: Arc::new(AtomicI64::new(at.timestamp_micros())) }
    }

    /// Move the clock to `at`.
    pub fn set(&self, at: DateTime<Utc>) {
        self.micros.store(at.timestamp_micros(), Ordering::SeqCst);
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        let micros = self.micros.load(Ordering::SeqCst);
        Utc.timestamp_micros(micros).single().unwrap_or_else(Utc::now)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn fixed_clock_returns_the_set_instant() {
        let t = Utc.with_ymd_and_hms(2026, 6, 15, 6, 0, 0).single().unwrap();
        let c = FixedClock::new(t);
        assert_eq!(c.now(), t);
    }

    #[test]
    fn fixed_clock_set_moves_now() {
        let t0 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).single().unwrap();
        let t1 = Utc.with_ymd_and_hms(2026, 6, 8, 6, 0, 0).single().unwrap();
        let c = FixedClock::new(t0);
        c.set(t1);
        assert_eq!(c.now(), t1);
    }

    #[test]
    fn fixed_clock_clones_share_the_same_instant() {
        let t0 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).single().unwrap();
        let c = FixedClock::new(t0);
        let c2 = c.clone();
        let t1 = Utc.with_ymd_and_hms(2026, 12, 31, 23, 0, 0).single().unwrap();
        c.set(t1);
        // The clone observes the update (shared atomic).
        assert_eq!(c2.now(), t1);
    }
}
