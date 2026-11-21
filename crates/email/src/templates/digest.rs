//! Daily reminder digest email.

use askama::Template;

use super::{Chrome, RenderedEmail, privacy_url};
use crate::locale::Locale;

// ── reminder digest ─────────────────────────────────────────────────────

#[derive(Template, Debug)]
#[template(path = "reminder_digest_en.txt", escape = "none")]
struct ReminderDigestEn<'a> {
    lead_days: i32,
    count: usize,
    lines: &'a [String],
    tree_link: &'a str,
    manage_link: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "reminder_digest_de.txt", escape = "none")]
struct ReminderDigestDe<'a> {
    lead_days: i32,
    count: usize,
    lines: &'a [String],
    tree_link: &'a str,
    manage_link: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "reminder_digest_en.html")]
struct ReminderDigestHtmlEn<'a> {
    chrome: Chrome<'a>,
    lead_days: i32,
    count: usize,
    lines: &'a [String],
    tree_link: &'a str,
    manage_link: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "reminder_digest_de.html")]
struct ReminderDigestHtmlDe<'a> {
    chrome: Chrome<'a>,
    lead_days: i32,
    count: usize,
    lines: &'a [String],
    tree_link: &'a str,
    manage_link: &'a str,
}

/// Inputs for the daily reminder digest.
///
/// `lines` are pre-rendered, localized one-liners (e.g. `"Anna — 40th
/// birthday"`); the worker builds them from the domain `UpcomingEvent`s so the
/// email crate stays free of projection logic.
#[derive(Debug)]
pub struct ReminderDigestArgs<'a> {
    pub lead_days: i32,
    pub lines: &'a [String],
    pub tree_link: &'a str,
    pub manage_link: &'a str,
}

/// Render the daily reminder digest for `locale`.
///
/// `args.lines` are the localized event lines; the subject summarizes the
/// count + lead time. An empty `lines` slice still renders, but callers
/// should skip sending when there are no events.
///
/// # Errors
/// Returns [`askama::Error`] if template rendering fails.
pub fn render_reminder_digest(
    locale: Locale,
    app_url: &str,
    args: &ReminderDigestArgs<'_>,
) -> Result<RenderedEmail, askama::Error> {
    let count = args.lines.len();
    let subject = match (locale, args.lead_days) {
        (Locale::De, 0) => {
            format!("🎂 Heute: {count} Familien-Termin{}", if count == 1 { "" } else { "e" })
        }
        (Locale::De, n) => {
            format!("🎂 In {n} Tagen: {count} Familien-Termin{}", if count == 1 { "" } else { "e" })
        }
        (Locale::En, 0) => {
            format!("🎂 Today: {count} family date{}", if count == 1 { "" } else { "s" })
        }
        (Locale::En, n) => {
            format!("🎂 In {n} days: {count} family date{}", if count == 1 { "" } else { "s" })
        }
    };
    let privacy = privacy_url(locale, app_url);
    let chrome = Chrome::new(locale, app_url, &privacy);
    let (text, html) = match locale {
        Locale::En => (
            ReminderDigestEn {
                lead_days: args.lead_days,
                count,
                lines: args.lines,
                tree_link: args.tree_link,
                manage_link: args.manage_link,
            }
            .render()?,
            ReminderDigestHtmlEn {
                chrome,
                lead_days: args.lead_days,
                count,
                lines: args.lines,
                tree_link: args.tree_link,
                manage_link: args.manage_link,
            }
            .render()?,
        ),
        Locale::De => (
            ReminderDigestDe {
                lead_days: args.lead_days,
                count,
                lines: args.lines,
                tree_link: args.tree_link,
                manage_link: args.manage_link,
            }
            .render()?,
            ReminderDigestHtmlDe {
                chrome,
                lead_days: args.lead_days,
                count,
                lines: args.lines,
                tree_link: args.tree_link,
                manage_link: args.manage_link,
            }
            .render()?,
        ),
    };
    Ok(RenderedEmail { subject, text, html })
}
