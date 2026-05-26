//! Pure upcoming-events projection shared by the `/upcoming` route and the reminder worker.
//!
//! Reads persons + partnerships for a family, projects each into the next
//! occurrence of its anniversary date, filters by kind + per-user favourites,
//! sorts ascending by date, then truncates.
//!
//! Presentation (the English label on the API, the localized digest line in
//! the worker) is built by each consumer from the structured fields — this
//! module is web- and locale-agnostic.

use std::collections::HashSet;
use std::sync::Arc;

use chrono::{Datelike, NaiveDate};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    FamilyId, Partnership, PartnershipRepo, Person, PersonFavouriteRepo, PersonId, PersonRepo,
    UserId,
};

/// The three event shapes the projection emits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpcomingKind {
    Birthday,
    WeddingAnniversary,
    DeathAnniversary,
}

/// One enumerated future date. `primary_name` is the person (birthday /
/// memorial) or the first partner (anniversary); `secondary_name` is the
/// second partner for anniversaries.
#[derive(Debug, Clone, Serialize)]
pub struct UpcomingEvent {
    pub kind: UpcomingKind,
    pub next_date: NaiveDate,
    pub years: u32,
    pub person_id: Option<Uuid>,
    pub partnership_id: Option<Uuid>,
    pub partner_a_id: Option<Uuid>,
    pub partner_b_id: Option<Uuid>,
    pub primary_name: String,
    pub secondary_name: Option<String>,
}

/// Default cap on rows the FE list ever displays.
pub const DEFAULT_LIMIT: u32 = 20;
/// Hard cap so a misbehaving client can't ask for a million rows.
pub const MAX_LIMIT: u32 = 200;
/// Cap on persons materialized per family — matches the tree service bound.
const MAX_PERSONS: u32 = 1_000;

/// Filter discriminator. `All` keeps everything, `Birthday` keeps only
/// birthdays, `Anniversary` keeps both wedding and death anniversaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpcomingFilter {
    All,
    Birthday,
    Anniversary,
}

impl UpcomingFilter {
    /// Parse the `?filter=` query parameter; unknown / missing → `All`.
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

/// Project a source month-day to the next occurrence on or after `today`.
/// Returns `None` for Feb-29 in a candidate non-leap year (chrono's
/// `with_year` returns `None`); the caller skips the event that cycle.
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
    match (g.is_empty(), f.is_empty()) {
        (false, false) => format!("{g} {f}"),
        (true, false) => f.to_owned(),
        _ => g.to_owned(),
    }
}

fn person_birthday(p: &Person, today: NaiveDate) -> Option<UpcomingEvent> {
    let birth = p.birth_date?;
    let next = next_occurrence(birth, today)?;
    let years = u32::try_from(next.year().saturating_sub(birth.year())).ok()?;
    Some(UpcomingEvent {
        kind: UpcomingKind::Birthday,
        next_date: next,
        years,
        person_id: Some(p.id.into_uuid()),
        partnership_id: None,
        partner_a_id: None,
        partner_b_id: None,
        primary_name: full_name(p),
        secondary_name: None,
    })
}

fn person_death_anniv(p: &Person, today: NaiveDate) -> Option<UpcomingEvent> {
    let death = p.death_date?;
    if death.year() == today.year() {
        return None;
    }
    let next = next_occurrence(death, today)?;
    let years = u32::try_from(next.year().saturating_sub(death.year())).ok()?;
    Some(UpcomingEvent {
        kind: UpcomingKind::DeathAnniversary,
        next_date: next,
        years,
        person_id: Some(p.id.into_uuid()),
        partnership_id: None,
        partner_a_id: None,
        partner_b_id: None,
        primary_name: full_name(p),
        secondary_name: None,
    })
}

fn wedding_anniv(part: &Partnership, persons: &[Person], today: NaiveDate) -> Option<UpcomingEvent> {
    if part.ended_on.is_some() {
        return None;
    }
    let started = part.started_on?;
    let next = next_occurrence(started, today)?;
    let years = u32::try_from(next.year().saturating_sub(started.year())).ok()?;
    let a = persons.iter().find(|p| p.id == part.partner_a_id);
    let b = persons.iter().find(|p| p.id == part.partner_b_id);
    Some(UpcomingEvent {
        kind: UpcomingKind::WeddingAnniversary,
        next_date: next,
        years,
        person_id: None,
        partnership_id: Some(part.id),
        partner_a_id: Some(part.partner_a_id.into_uuid()),
        partner_b_id: Some(part.partner_b_id.into_uuid()),
        primary_name: a.map_or_else(String::new, full_name),
        secondary_name: b.map(full_name),
    })
}

/// Build the upcoming-events list for `family_id`, filtered by kind +
/// per-user favourites, sorted ascending by `next_date`, truncated to `limit`.
///
/// # Errors
/// Propagates any error surfaced by the underlying repos (DB connectivity,
/// query failure).
#[allow(
    clippy::too_many_arguments,
    reason = "projection orchestrator: explicit repo+scope+filter args keep call sites self-documenting"
)]
pub async fn build_upcoming(
    persons: &Arc<dyn PersonRepo>,
    partnerships: &Arc<dyn PartnershipRepo>,
    favourites: &Arc<dyn PersonFavouriteRepo>,
    family_id: FamilyId,
    user_id: UserId,
    today: NaiveDate,
    filter: UpcomingFilter,
    favourites_only: bool,
    limit: u32,
) -> anyhow::Result<Vec<UpcomingEvent>> {
    let people = persons.list_for_family(family_id, None, MAX_PERSONS).await?;
    let parts = partnerships.list_for_family(family_id).await?;
    // Resolve the favourite set up front so the per-event check is O(1). When
    // the filter is disabled we never read it, so skip the round trip.
    let fav_set: HashSet<PersonId> = if favourites_only {
        favourites.list_for_user(user_id, family_id).await?
    } else {
        HashSet::new()
    };

    let keep_person = |id: PersonId| !favourites_only || fav_set.contains(&id);
    let keep_partnership =
        |a: PersonId, b: PersonId| !favourites_only || fav_set.contains(&a) || fav_set.contains(&b);

    let mut events: Vec<UpcomingEvent> = Vec::new();
    if filter.keeps_birthday() {
        for p in &people {
            if !keep_person(p.id) {
                continue;
            }
            if let Some(ev) = person_birthday(p, today) {
                events.push(ev);
            }
        }
    }
    if filter.keeps_anniversary() {
        for p in &people {
            if !keep_person(p.id) {
                continue;
            }
            if let Some(ev) = person_death_anniv(p, today) {
                events.push(ev);
            }
        }
        for part in &parts {
            if !keep_partnership(part.partner_a_id, part.partner_b_id) {
                continue;
            }
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
    fn next_occurrence_uses_this_year_when_still_ahead() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 23).unwrap();
        let source = NaiveDate::from_ymd_opt(1980, 7, 4).unwrap();
        assert_eq!(
            next_occurrence(source, today).unwrap(),
            NaiveDate::from_ymd_opt(2026, 7, 4).unwrap()
        );
    }

    #[test]
    fn next_occurrence_rolls_to_next_year_when_passed() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 23).unwrap();
        let source = NaiveDate::from_ymd_opt(1980, 4, 15).unwrap();
        assert_eq!(
            next_occurrence(source, today).unwrap(),
            NaiveDate::from_ymd_opt(2027, 4, 15).unwrap()
        );
    }

    #[test]
    fn next_occurrence_keeps_today_itself() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 23).unwrap();
        let source = NaiveDate::from_ymd_opt(1990, 5, 23).unwrap();
        assert_eq!(next_occurrence(source, today).unwrap(), today);
    }
}
