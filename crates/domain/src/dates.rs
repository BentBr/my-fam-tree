//! Small `time` helpers + constants shared across crates. No I/O.
//!
//! Timezone-aware conversion (IANA zones, DST) is deliberately NOT done in
//! Rust: Postgres ships the tz database, so the persistence layer resolves
//! local times there (see `TimeZoneRepo` and
//! `ReminderPreferencesRepo::due_at_local_hour`).

use time::{Date, Month};

/// Zone used when a user's stored `timezone` is not a valid IANA name.
pub const DEFAULT_TIMEZONE: &str = "Europe/Berlin";

/// Local hour (0–23) at which the daily reminder digest is scheduled.
pub const DIGEST_LOCAL_HOUR: u8 = 6;

/// Build a calendar date from numeric parts; `None` when the combination is
/// not a real date (month 13, Feb 30, Feb 29 in a non-leap year, …).
#[must_use]
pub const fn date_from_ymd(year: i32, month: u8, day: u8) -> Option<Date> {
    let month = match month {
        1 => Month::January,
        2 => Month::February,
        3 => Month::March,
        4 => Month::April,
        5 => Month::May,
        6 => Month::June,
        7 => Month::July,
        8 => Month::August,
        9 => Month::September,
        10 => Month::October,
        11 => Month::November,
        12 => Month::December,
        _ => return None,
    };
    match Date::from_calendar_date(year, month, day) {
        Ok(d) => Some(d),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn valid_date_round_trips() {
        let d = date_from_ymd(2026, 6, 15).unwrap();
        assert_eq!((d.year(), u8::from(d.month()), d.day()), (2026, 6, 15));
    }

    #[test]
    fn invalid_month_is_none() {
        assert!(date_from_ymd(2026, 0, 1).is_none());
        assert!(date_from_ymd(2026, 13, 1).is_none());
    }

    #[test]
    fn invalid_day_is_none() {
        assert!(date_from_ymd(2026, 2, 30).is_none());
        assert!(date_from_ymd(2026, 4, 31).is_none());
    }

    #[test]
    fn every_month_maps_to_itself() {
        for m in 1..=12_u8 {
            assert_eq!(u8::from(date_from_ymd(2026, m, 1).unwrap().month()), m);
        }
    }

    #[test]
    fn leap_day_only_in_leap_years() {
        assert!(date_from_ymd(2024, 2, 29).is_some());
        assert!(date_from_ymd(2026, 2, 29).is_none());
        assert!(date_from_ymd(1900, 2, 29).is_none());
        assert!(date_from_ymd(2000, 2, 29).is_some());
    }
}
