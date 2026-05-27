//! Wire DTO + label rendering for `GET /api/v1/upcoming`.
//!
//! The projection itself lives in [`my_family_domain::upcoming`] so the
//! reminder worker can reuse it. This module maps the domain event to the
//! API's `ToSchema` shape and renders the English `label` server-side (the FE
//! shows it verbatim; i18n stays on `upcoming.kinds.*` for the chips).

use chrono::NaiveDate;
pub use my_family_domain::{DEFAULT_LIMIT, MAX_LIMIT, UpcomingFilter, build_upcoming};
use my_family_domain::{UpcomingEvent as DomainEvent, UpcomingKind};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

/// One enumerated future date as sent on the wire.
///
/// `kind` is one of `birthday`, `wedding_anniversary`, `death_anniversary`.
/// `person_id` is set for `birthday`/`death_anniversary`; `partnership_id`
/// (+ both partner ids) for `wedding_anniversary`. `label` is pre-rendered
/// server-side so the FE doesn't re-derive the "Nth birthday" phrasing.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UpcomingEvent {
    pub kind: String,
    pub next_date: NaiveDate,
    pub years: u32,
    pub person_id: Option<Uuid>,
    pub partnership_id: Option<Uuid>,
    pub partner_a_id: Option<Uuid>,
    pub partner_b_id: Option<Uuid>,
    pub label: String,
}

/// English ordinal suffix; 11/12/13 always `th`, otherwise by last digit.
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

const fn kind_str(k: UpcomingKind) -> &'static str {
    match k {
        UpcomingKind::Birthday => "birthday",
        UpcomingKind::WeddingAnniversary => "wedding_anniversary",
        UpcomingKind::DeathAnniversary => "death_anniversary",
    }
}

impl From<DomainEvent> for UpcomingEvent {
    fn from(e: DomainEvent) -> Self {
        let label = match e.kind {
            UpcomingKind::Birthday => {
                format!("{} — {}{} birthday", e.primary_name, e.years, ordinal_suffix(e.years))
            }
            UpcomingKind::DeathAnniversary => {
                format!("{} — {}{} memorial", e.primary_name, e.years, ordinal_suffix(e.years))
            }
            UpcomingKind::WeddingAnniversary => {
                let pair = match e.secondary_name.as_deref() {
                    Some(b) if !b.is_empty() && !e.primary_name.is_empty() => {
                        format!("{} & {}", e.primary_name, b)
                    }
                    Some(b) if e.primary_name.is_empty() => b.to_owned(),
                    _ if !e.primary_name.is_empty() => e.primary_name.clone(),
                    _ => "Partnership".to_owned(),
                };
                format!("{pair} — {}{} anniversary", e.years, ordinal_suffix(e.years))
            }
        };
        Self {
            kind: kind_str(e.kind).to_owned(),
            next_date: e.next_date,
            years: e.years,
            person_id: e.person_id,
            partnership_id: e.partnership_id,
            partner_a_id: e.partner_a_id,
            partner_b_id: e.partner_b_id,
            label,
        }
    }
}

#[cfg(test)]
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
}
