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

use time::OffsetDateTime;

/// Microseconds since the Unix epoch, saturating at the `i64` range.
fn to_micros(t: OffsetDateTime) -> i64 {
    i64::try_from(t.unix_timestamp_nanos() / 1_000).unwrap_or(i64::MAX)
}

/// Inverse of [`to_micros`]; falls back to the wall clock for an
/// out-of-range value (never happens for real instants).
fn from_micros(micros: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(micros).saturating_mul(1_000))
        .unwrap_or_else(|_| OffsetDateTime::now_utc())
}

/// Source of "now". Production uses [`SystemClock`]; tests + the
/// `test-fixtures` HTTP listener use [`FixedClock`].
pub trait Clock: Send + Sync + std::fmt::Debug {
    fn now(&self) -> OffsetDateTime;
}

#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
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
    pub fn new(at: OffsetDateTime) -> Self {
        Self { micros: Arc::new(AtomicI64::new(to_micros(at))) }
    }

    /// Move the clock to `at`.
    pub fn set(&self, at: OffsetDateTime) {
        self.micros.store(to_micros(at), Ordering::SeqCst);
    }
}

impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        let micros = self.micros.load(Ordering::SeqCst);
        from_micros(micros)
    }
}

/// Real-time clock with a settable forward/backward offset.
///
/// `now() = OffsetDateTime::now_utc() + offset`, where `offset` starts at zero (so the
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
    /// Microseconds to add to `OffsetDateTime::now_utc()` to produce the clock's `now`.
    offset_micros: Arc<AtomicI64>,
}

impl Default for OffsetClock {
    fn default() -> Self {
        Self::new(OffsetDateTime::now_utc())
    }
}

impl OffsetClock {
    /// Create a clock whose `now()` is approximately `at` (within the
    /// time it takes to read `OffsetDateTime::now_utc()`).
    #[must_use]
    pub fn new(at: OffsetDateTime) -> Self {
        let offset = to_micros(at).saturating_sub(to_micros(OffsetDateTime::now_utc()));
        Self { offset_micros: Arc::new(AtomicI64::new(offset)) }
    }

    /// Move the clock to approximately `at`. The clock then continues
    /// advancing with real wall time from `at`.
    pub fn set(&self, at: OffsetDateTime) {
        let offset = to_micros(at).saturating_sub(to_micros(OffsetDateTime::now_utc()));
        self.offset_micros.store(offset, Ordering::SeqCst);
    }
}

impl Clock for OffsetClock {
    fn now(&self) -> OffsetDateTime {
        let offset = self.offset_micros.load(Ordering::SeqCst);
        let micros = to_micros(OffsetDateTime::now_utc()).saturating_add(offset);
        from_micros(micros)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_clock_returns_the_set_instant() {
        let t = time::macros::datetime!(2026-06-15 06:00:00 UTC);
        let c = FixedClock::new(t);
        assert_eq!(c.now(), t);
    }

    #[test]
    fn fixed_clock_set_moves_now() {
        let t0 = time::macros::datetime!(2026-01-01 00:00:00 UTC);
        let t1 = time::macros::datetime!(2026-06-08 06:00:00 UTC);
        let c = FixedClock::new(t0);
        c.set(t1);
        assert_eq!(c.now(), t1);
    }

    #[test]
    fn fixed_clock_clones_share_the_same_instant() {
        let t0 = time::macros::datetime!(2026-01-01 00:00:00 UTC);
        let c = FixedClock::new(t0);
        let c2 = c.clone();
        let t1 = time::macros::datetime!(2026-12-31 23:00:00 UTC);
        c.set(t1);
        // The clone observes the update (shared atomic).
        assert_eq!(c2.now(), t1);
    }

    #[test]
    fn offset_clock_default_tracks_real_time() {
        // No `set(...)` call → offset = 0 → behaves like `SystemClock`.
        // Allow a small tolerance for the time between `OffsetDateTime::now_utc()` reads.
        let c = OffsetClock::default();
        let real = OffsetDateTime::now_utc();
        let diff = (c.now() - real).whole_milliseconds().abs();
        assert!(
            diff < 100,
            "OffsetClock should track OffsetDateTime::now_utc() within 100 ms; got {diff} ms"
        );
    }

    #[test]
    fn offset_clock_set_jumps_then_advances() {
        let target = time::macros::datetime!(2026-06-08 04:00:00 UTC);
        let c = OffsetClock::default();
        c.set(target);
        // Immediately after `set`, `now()` is within a few ms of `target`.
        let drift_a = (c.now() - target).whole_milliseconds().abs();
        assert!(drift_a < 100, "post-set drift {drift_a} ms exceeds tolerance");
        // After a short real-time sleep, `now()` has advanced by ~the same
        // amount — the clock isn't frozen.
        std::thread::sleep(std::time::Duration::from_millis(50));
        let advanced = c.now() - target;
        assert!(
            advanced.whole_milliseconds() >= 30,
            "OffsetClock did not advance with real time (got {advanced})",
        );
    }
}
