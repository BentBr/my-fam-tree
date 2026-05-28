//! Digest projection + line rendering.
//!
//! Turns a user's preferences into the events that fall on a target date,
//! across all their families, then renders localized digest lines. Reuses
//! [`my_family_domain::build_upcoming`] — the SAME projection the `/upcoming`
//! route uses — so the digest can never drift from the Upcoming page.

use chrono::NaiveDate;
use my_family_domain::{
    MAX_LIMIT, ReminderPreferences, UpcomingEvent, UpcomingFilter, UpcomingKind, UserId,
    build_upcoming,
};
use my_family_email::Locale;

use crate::state::WorkerState;

/// Map the two kind toggles onto the shared filter enum. `None` means the user
/// wants neither kind — the caller skips them entirely.
#[must_use]
pub const fn filter_from_prefs(p: &ReminderPreferences) -> Option<UpcomingFilter> {
    match (p.remind_birthdays, p.remind_anniversaries) {
        (true, true) => Some(UpcomingFilter::All),
        (true, false) => Some(UpcomingFilter::Birthday),
        (false, true) => Some(UpcomingFilter::Anniversary),
        (false, false) => None,
    }
}

/// All events for `user` occurring exactly on `target_date`, across every
/// family they belong to, honouring their kind toggles + favourites scope.
///
/// # Errors
/// Propagates repo errors from membership lookup or `build_upcoming`.
pub async fn events_for_user_on(
    state: &WorkerState,
    user_id: UserId,
    prefs: &ReminderPreferences,
    today: NaiveDate,
    target_date: NaiveDate,
) -> anyhow::Result<Vec<UpcomingEvent>> {
    let Some(filter) = filter_from_prefs(prefs) else { return Ok(Vec::new()) };
    let mut out: Vec<UpcomingEvent> = Vec::new();
    for m in state.memberships.list_for_user(user_id).await? {
        let events = build_upcoming(
            &state.persons,
            &state.partnerships,
            &state.favourites,
            m.family_id,
            user_id,
            today,
            filter,
            prefs.favourites_only,
            MAX_LIMIT,
        )
        .await?;
        out.extend(events.into_iter().filter(|e| e.next_date == target_date));
    }
    out.sort_by(|a, b| a.primary_name.cmp(&b.primary_name));
    Ok(out)
}

/// Render one localized digest line from an event, e.g. `"Anna — 40th
/// birthday"` (en) / `"Anna — 40. Geburtstag"` (de).
#[must_use]
pub fn render_line(locale: Locale, e: &UpcomingEvent) -> String {
    match locale {
        Locale::En => render_line_en(e),
        Locale::De => render_line_de(e),
    }
}

fn ordinal_en(n: u32) -> &'static str {
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

fn pair(e: &UpcomingEvent) -> String {
    match e.secondary_name.as_deref() {
        Some(b) if !b.is_empty() && !e.primary_name.is_empty() => {
            format!("{} & {}", e.primary_name, b)
        }
        Some(b) if e.primary_name.is_empty() => b.to_owned(),
        _ => e.primary_name.clone(),
    }
}

fn render_line_en(e: &UpcomingEvent) -> String {
    match e.kind {
        UpcomingKind::Birthday => {
            format!("{} — {}{} birthday", e.primary_name, e.years, ordinal_en(e.years))
        }
        UpcomingKind::DeathAnniversary => {
            format!("{} — {}{} memorial", e.primary_name, e.years, ordinal_en(e.years))
        }
        UpcomingKind::WeddingAnniversary => {
            format!("{} — {}{} anniversary", pair(e), e.years, ordinal_en(e.years))
        }
    }
}

fn render_line_de(e: &UpcomingEvent) -> String {
    match e.kind {
        UpcomingKind::Birthday => format!("{} — {}. Geburtstag", e.primary_name, e.years),
        UpcomingKind::DeathAnniversary => format!("{} — {}. Gedenktag", e.primary_name, e.years),
        UpcomingKind::WeddingAnniversary => format!("{} — {}. Jahrestag", pair(e), e.years),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn prefs(birthdays: bool, anniversaries: bool) -> ReminderPreferences {
        ReminderPreferences {
            emails_enabled: true,
            remind_birthdays: birthdays,
            remind_anniversaries: anniversaries,
            favourites_only: false,
            lead_days: 7,
        }
    }

    fn event(
        kind: UpcomingKind,
        primary: &str,
        secondary: Option<&str>,
        years: u32,
    ) -> UpcomingEvent {
        UpcomingEvent {
            kind,
            next_date: NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
            years,
            person_id: None,
            partnership_id: None,
            partner_a_id: None,
            partner_b_id: None,
            primary_name: primary.to_owned(),
            secondary_name: secondary.map(ToOwned::to_owned),
        }
    }

    #[test]
    fn filter_from_prefs_covers_all_four_combinations() {
        assert_eq!(filter_from_prefs(&prefs(true, true)), Some(UpcomingFilter::All));
        assert_eq!(filter_from_prefs(&prefs(true, false)), Some(UpcomingFilter::Birthday));
        assert_eq!(filter_from_prefs(&prefs(false, true)), Some(UpcomingFilter::Anniversary));
        assert_eq!(filter_from_prefs(&prefs(false, false)), None);
    }

    #[test]
    fn render_line_en_birthday_uses_ordinal() {
        let e = event(UpcomingKind::Birthday, "Anna Müller", None, 40);
        assert_eq!(render_line(Locale::En, &e), "Anna Müller — 40th birthday");
        let e21 = event(UpcomingKind::Birthday, "Bo", None, 21);
        assert_eq!(render_line(Locale::En, &e21), "Bo — 21st birthday");
        let e13 = event(UpcomingKind::Birthday, "Cy", None, 13);
        assert_eq!(render_line(Locale::En, &e13), "Cy — 13th birthday");
    }

    #[test]
    fn render_line_en_wedding_joins_partners() {
        let e = event(UpcomingKind::WeddingAnniversary, "Anna", Some("Klaus"), 10);
        assert_eq!(render_line(Locale::En, &e), "Anna & Klaus — 10th anniversary");
    }

    #[test]
    fn render_line_en_memorial() {
        let e = event(UpcomingKind::DeathAnniversary, "Otto", None, 5);
        assert_eq!(render_line(Locale::En, &e), "Otto — 5th memorial");
    }

    #[test]
    fn render_line_de_all_kinds() {
        assert_eq!(
            render_line(Locale::De, &event(UpcomingKind::Birthday, "Anna", None, 40)),
            "Anna — 40. Geburtstag"
        );
        assert_eq!(
            render_line(
                Locale::De,
                &event(UpcomingKind::WeddingAnniversary, "Anna", Some("Klaus"), 10)
            ),
            "Anna & Klaus — 10. Jahrestag"
        );
        assert_eq!(
            render_line(Locale::De, &event(UpcomingKind::DeathAnniversary, "Otto", None, 5)),
            "Otto — 5. Gedenktag"
        );
    }

    #[test]
    fn render_line_wedding_falls_back_to_single_partner_when_secondary_missing() {
        let e = event(UpcomingKind::WeddingAnniversary, "Anna", None, 3);
        assert_eq!(render_line(Locale::En, &e), "Anna — 3rd anniversary");
    }
}
