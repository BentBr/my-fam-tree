//! Upcoming-dates orchestration for `GET /api/v1/upcoming`.
//!
//! Pulls all persons + partnerships for a family, projects each
//! source into a list of `UpcomingEvent` rows (birthday, wedding
//! anniversary, death anniversary), then filters / sorts / limits.
//!
//! "Next occurrence" maps a `NaiveDate` (the month-day of birth /
//! death / marriage) to the upcoming anniversary relative to `today`:
//! - if this year's anniversary is today or later → use this year.
//! - else → use next year.
//!
//! `years` is `next_date.year() - source_date.year()`. For a birthday
//! it reads as "they will turn N"; for an anniversary it reads as
//! "Nth anniversary".

use std::sync::Arc;

use chrono::{Datelike, NaiveDate};
use my_family_domain::{FamilyId, Partnership, PartnershipRepo, Person, PersonRepo};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

/// One enumerated future date.
///
/// `kind` is one of `birthday`, `wedding_anniversary`, `death_anniversary`.
/// `person_id` is set for `birthday` and `death_anniversary`;
/// `partnership_id` is set for `wedding_anniversary`.
/// `label` is pre-rendered server-side so the FE doesn't need to
/// re-translate the "Nth birthday" phrasing per locale (i18n stays
/// on `upcoming.kinds.*`; the name + N come from the API).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UpcomingEvent {
    pub kind: String,
    pub next_date: NaiveDate,
    pub years: u32,
    pub person_id: Option<Uuid>,
    pub partnership_id: Option<Uuid>,
    pub label: String,
}

/// Cap on rows the FE list ever displays. Matches the default the
/// route's `limit` query parameter advertises.
pub const DEFAULT_LIMIT: u32 = 20;
/// Hard cap so a misbehaving client can't ask for a million rows.
pub const MAX_LIMIT: u32 = 200;
/// Cap on persons we materialize per family. The same bound the
/// relationships-tree service uses; larger families fall outside the
/// MVP scope.
const MAX_PERSONS: u32 = 1_000;

/// Filter discriminator. `All` keeps everything, `Birthday` keeps
/// only `birthday` events, `Anniversary` keeps both wedding and
/// death anniversaries (the German "Jahrestag" idiom).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpcomingFilter {
    All,
    Birthday,
    Anniversary,
}

impl UpcomingFilter {
    /// Parse the `?filter=` query parameter.
    ///
    /// Accepts `all`, `birthday`, `anniversary`; defaults to `All` for
    /// an empty/missing input. Unknown values fall back to `All` so
    /// the endpoint stays forgiving — invalid filter strings already
    /// produce 422 in the route layer via `value_required` if we want
    /// stricter behaviour later.
    #[must_use]
    pub fn parse(raw: Option<&str>) -> Self {
        match raw {
            Some("birthday") => Self::Birthday,
            Some("anniversary") => Self::Anniversary,
            _ => Self::All,
        }
    }

    const fn keeps_birthday(self) -> bool {
        matches!(self, Self::All | Self::Birthday)
    }

    const fn keeps_anniversary(self) -> bool {
        matches!(self, Self::All | Self::Anniversary)
    }
}

/// Project a source `NaiveDate` (someone's birthday, the wedding
/// date, etc.) to the next occurrence on or after `today`.
///
/// Returns `None` if `source` falls in a year where `feb-29` is not
/// representable in the candidate year — chrono returns `None` from
/// `with_year(_)` and we propagate that. The caller treats the event
/// as "no next occurrence this cycle" and skips it.
#[must_use]
fn next_occurrence(source: NaiveDate, today: NaiveDate) -> Option<NaiveDate> {
    let this_year = source.with_year(today.year())?;
    if this_year >= today {
        Some(this_year)
    } else {
        source.with_year(today.year().saturating_add(1))
    }
}

fn full_name(p: &Person) -> String {
    let g = p.given_name.trim();
    let f = p.family_name.trim();
    if f.is_empty() {
        g.to_owned()
    } else if g.is_empty() {
        f.to_owned()
    } else {
        format!("{g} {f}")
    }
}

fn label_birthday(p: &Person, years: u32) -> String {
    format!("{} — {years}{} birthday", full_name(p), ordinal_suffix(years))
}

fn label_death_anniv(p: &Person, years: u32) -> String {
    format!("{} — {years}{} memorial", full_name(p), ordinal_suffix(years))
}

fn label_wedding_anniv(a: Option<&Person>, b: Option<&Person>, years: u32) -> String {
    let a_name = a.map_or_else(String::new, full_name);
    let b_name = b.map_or_else(String::new, full_name);
    let pair = match (a_name.is_empty(), b_name.is_empty()) {
        (false, false) => format!("{a_name} & {b_name}"),
        (false, true) => a_name,
        (true, false) => b_name,
        (true, true) => String::from("Partnership"),
    };
    format!("{pair} — {years}{} anniversary", ordinal_suffix(years))
}

/// English ordinal suffix for the count. Numbers ending in 11/12/13
/// always get `th`; 1/2/3 (any other position) get `st`/`nd`/`rd`.
fn ordinal_suffix(n: u32) -> &'static str {
    let last_two = n % 100;
    if (11..=13).contains(&last_two) {
        return "th";
    }
    match n % 10 {
        1 => "st",
        2 => "nd",
        3 => "rd",
        _ => "th",
    }
}

/// Build a single person's birthday event when the row carries a
/// `birth_date`. Returns `None` when there's no `birth_date` or when
/// the next-occurrence projection fails (feb-29 in a non-leap year).
fn person_birthday(p: &Person, today: NaiveDate) -> Option<UpcomingEvent> {
    let birth = p.birth_date?;
    let next = next_occurrence(birth, today)?;
    let years = u32::try_from(next.year().saturating_sub(birth.year())).ok()?;
    Some(UpcomingEvent {
        kind: "birthday".to_owned(),
        next_date: next,
        years,
        person_id: Some(p.id.into_uuid()),
        partnership_id: None,
        label: label_birthday(p, years),
    })
}

/// Death anniversary. Skipped in the year of death itself so the FE
/// doesn't show a "0th memorial" the same year someone passes away.
fn person_death_anniv(p: &Person, today: NaiveDate) -> Option<UpcomingEvent> {
    let death = p.death_date?;
    if death.year() == today.year() {
        return None;
    }
    let next = next_occurrence(death, today)?;
    let years = u32::try_from(next.year().saturating_sub(death.year())).ok()?;
    Some(UpcomingEvent {
        kind: "death_anniversary".to_owned(),
        next_date: next,
        years,
        person_id: Some(p.id.into_uuid()),
        partnership_id: None,
        label: label_death_anniv(p, years),
    })
}

/// Wedding anniversary for an open partnership with a `started_on`.
/// Closed partnerships (`ended_on IS NOT NULL`) never emit an event.
fn wedding_anniv(
    part: &Partnership,
    persons: &[Person],
    today: NaiveDate,
) -> Option<UpcomingEvent> {
    if part.ended_on.is_some() {
        return None;
    }
    let started = part.started_on?;
    let next = next_occurrence(started, today)?;
    let years = u32::try_from(next.year().saturating_sub(started.year())).ok()?;
    let a = persons.iter().find(|p| p.id == part.partner_a_id);
    let b = persons.iter().find(|p| p.id == part.partner_b_id);
    Some(UpcomingEvent {
        kind: "wedding_anniversary".to_owned(),
        next_date: next,
        years,
        person_id: None,
        partnership_id: Some(part.id),
        label: label_wedding_anniv(a, b, years),
    })
}

/// Build the upcoming-events list for `family_id`, filter it, sort
/// ascending by `next_date`, then truncate to `limit`.
///
/// # Errors
/// Returns any error surfaced by the underlying repos (DB
/// connectivity, query failure).
pub async fn build_upcoming(
    persons: &Arc<dyn PersonRepo>,
    partnerships: &Arc<dyn PartnershipRepo>,
    family_id: FamilyId,
    today: NaiveDate,
    filter: UpcomingFilter,
    limit: u32,
) -> anyhow::Result<Vec<UpcomingEvent>> {
    let people = persons.list_for_family(family_id, None, MAX_PERSONS).await?;
    let parts = partnerships.list_for_family(family_id).await?;

    let mut events: Vec<UpcomingEvent> = Vec::new();
    if filter.keeps_birthday() {
        for p in &people {
            if let Some(ev) = person_birthday(p, today) {
                events.push(ev);
            }
        }
    }
    if filter.keeps_anniversary() {
        for p in &people {
            if let Some(ev) = person_death_anniv(p, today) {
                events.push(ev);
            }
        }
        for part in &parts {
            if let Some(ev) = wedding_anniv(part, &people, today) {
                events.push(ev);
            }
        }
    }

    events.sort_by_key(|e| e.next_date);
    let cap = usize::try_from(limit.min(MAX_LIMIT)).unwrap_or(usize::MAX);
    events.truncate(cap);
    Ok(events)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn ordinal_suffix_handles_teens_and_units() {
        assert_eq!(ordinal_suffix(1), "st");
        assert_eq!(ordinal_suffix(2), "nd");
        assert_eq!(ordinal_suffix(3), "rd");
        assert_eq!(ordinal_suffix(4), "th");
        assert_eq!(ordinal_suffix(11), "th");
        assert_eq!(ordinal_suffix(12), "th");
        assert_eq!(ordinal_suffix(13), "th");
        assert_eq!(ordinal_suffix(21), "st");
        assert_eq!(ordinal_suffix(112), "th");
    }

    #[test]
    fn next_occurrence_uses_this_year_when_still_ahead() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 23).unwrap();
        let source = NaiveDate::from_ymd_opt(1980, 7, 4).unwrap();
        let got = next_occurrence(source, today).unwrap();
        assert_eq!(got, NaiveDate::from_ymd_opt(2026, 7, 4).unwrap());
    }

    #[test]
    fn next_occurrence_rolls_to_next_year_when_passed() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 23).unwrap();
        let source = NaiveDate::from_ymd_opt(1980, 4, 15).unwrap();
        let got = next_occurrence(source, today).unwrap();
        assert_eq!(got, NaiveDate::from_ymd_opt(2027, 4, 15).unwrap());
    }

    #[test]
    fn next_occurrence_keeps_today_itself() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 23).unwrap();
        let source = NaiveDate::from_ymd_opt(1990, 5, 23).unwrap();
        let got = next_occurrence(source, today).unwrap();
        assert_eq!(got, today);
    }
}
