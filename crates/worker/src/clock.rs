//! Clock abstraction so the worker's "now" is injectable in tests + E2E.
//!
//! Two implementations of [`Clock`] live here:
//!
//! - [`FixedClock`] — frozen at a stored instant. Used by deterministic
//!   unit tests where reads must repeatedly observe the same `now`.
//! - [`OffsetClock`] — wall-clock plus a settable offset. Used by the live
//!   `test-fixtures` worker binary (e2e). A frozen clock breaks the
//!   outbox poller: rows the api inserts with `next_attempt_at = db now()`
//!   are always "in the future" relative to a frozen worker clock, so they
//!   never become due. `OffsetClock` advances with real time between
//!   `set(...)` calls, so the outbox drains normally while the digest
//!   ticker can still be fast-forwarded.

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

/// Real-time clock with a settable forward/backward offset.
///
/// `now() = Utc::now() + offset`, where `offset` starts at zero (so the
/// clock behaves identically to [`SystemClock`] until something calls
/// [`set`](Self::set)). Each `set(t)` snapshots the current wall clock and
/// stores `offset = t - real_now`, so the clock immediately reads `t` and
/// then advances with real time from there.
///
/// This is what the `test-fixtures` HTTP listener wires into
/// [`WorkerState`](crate::state::WorkerState). The e2e suite can still
/// fast-forward the worker to a future date for the digest test, while
/// the outbox poller keeps draining magic-link rows the api inserts at
/// real time (a frozen [`FixedClock`] would leave those rows undue
/// forever).
#[derive(Debug, Clone)]
pub struct OffsetClock {
    /// Microseconds to add to `Utc::now()` to produce the clock's `now`.
    offset_micros: Arc<AtomicI64>,
}

impl Default for OffsetClock {
    fn default() -> Self {
        Self::new(Utc::now())
    }
}

impl OffsetClock {
    /// Create a clock whose `now()` is approximately `at` (within the
    /// time it takes to read `Utc::now()`).
    #[must_use]
    pub fn new(at: DateTime<Utc>) -> Self {
        let offset = at.timestamp_micros().saturating_sub(Utc::now().timestamp_micros());
        Self { offset_micros: Arc::new(AtomicI64::new(offset)) }
    }

    /// Move the clock to approximately `at`. The clock then continues
    /// advancing with real wall time from `at`.
    pub fn set(&self, at: DateTime<Utc>) {
        let offset = at.timestamp_micros().saturating_sub(Utc::now().timestamp_micros());
        self.offset_micros.store(offset, Ordering::SeqCst);
    }
}

impl Clock for OffsetClock {
    fn now(&self) -> DateTime<Utc> {
        let offset = self.offset_micros.load(Ordering::SeqCst);
        let micros = Utc::now().timestamp_micros().saturating_add(offset);
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

    #[test]
    fn offset_clock_default_tracks_real_time() {
        // No `set(...)` call → offset = 0 → behaves like `SystemClock`.
        // Allow a small tolerance for the time between `Utc::now()` reads.
        let c = OffsetClock::default();
        let real = Utc::now();
        let diff = (c.now() - real).num_milliseconds().abs();
        assert!(diff < 100, "OffsetClock should track Utc::now() within 100 ms; got {diff} ms");
    }

    #[test]
    fn offset_clock_set_jumps_then_advances() {
        let target = Utc.with_ymd_and_hms(2026, 6, 8, 4, 0, 0).single().unwrap();
        let c = OffsetClock::default();
        c.set(target);
        // Immediately after `set`, `now()` is within a few ms of `target`.
        let drift_a = (c.now() - target).num_milliseconds().abs();
        assert!(drift_a < 100, "post-set drift {drift_a} ms exceeds tolerance");
        // After a short real-time sleep, `now()` has advanced by ~the same
        // amount — the clock isn't frozen.
        std::thread::sleep(std::time::Duration::from_millis(50));
        let advanced = c.now() - target;
        assert!(
            advanced.num_milliseconds() >= 30,
            "OffsetClock did not advance with real time (got {advanced})",
        );
    }
}
