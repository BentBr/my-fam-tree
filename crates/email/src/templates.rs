//! Locale-aware rendering of plain-text email bodies.
//!
//! The templates live in `crates/email/templates/` (askama's default lookup
//! directory) and are compiled into the binary via `#[derive(Template)]`. We
//! disable HTML escaping because every body here is plain text and the
//! variables we interpolate (URLs, names) are already trusted user input
//! that's gone through validation upstream.
//!
//! Each `render_*` helper returns `(subject, body)` so call-sites can hand
//! both straight to [`crate::sender::OutboundEmail`].

use askama::Template;

use crate::locale::Locale;

#[derive(Template, Debug)]
#[template(path = "magic_link_en.txt", escape = "none")]
struct MagicLinkEn<'a> {
    link: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "magic_link_de.txt", escape = "none")]
struct MagicLinkDe<'a> {
    link: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "invite_en.txt", escape = "none")]
struct InviteEn<'a> {
    link: &'a str,
    inviter_name: &'a str,
    family_name: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "invite_de.txt", escape = "none")]
struct InviteDe<'a> {
    link: &'a str,
    inviter_name: &'a str,
    family_name: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "email_change_en.txt", escape = "none")]
struct EmailChangeEn<'a> {
    link: &'a str,
    new_email: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "email_change_de.txt", escape = "none")]
struct EmailChangeDe<'a> {
    link: &'a str,
    new_email: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "owner_transfer_owner_en.txt", escape = "none")]
struct OwnerTransferOwnerEn<'a> {
    family_name: &'a str,
    to_user_display_name: &'a str,
    link: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "owner_transfer_owner_de.txt", escape = "none")]
struct OwnerTransferOwnerDe<'a> {
    family_name: &'a str,
    to_user_display_name: &'a str,
    link: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "owner_transfer_admin_en.txt", escape = "none")]
struct OwnerTransferAdminEn<'a> {
    family_name: &'a str,
    from_user_display_name: &'a str,
    to_user_display_name: &'a str,
    link: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "owner_transfer_admin_de.txt", escape = "none")]
struct OwnerTransferAdminDe<'a> {
    family_name: &'a str,
    from_user_display_name: &'a str,
    to_user_display_name: &'a str,
    link: &'a str,
}

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

/// Render the magic-link sign-in email for `locale`.
///
/// Returns `(subject, body)`. Errors propagate from askama (template logic
/// errors only; the template files themselves are compiled in).
///
/// # Errors
/// Returns [`askama::Error`] if template rendering fails.
pub fn render_magic_link(locale: Locale, link: &str) -> Result<(String, String), askama::Error> {
    let (subject, body) = match locale {
        Locale::En => ("Sign in to my-family".to_string(), MagicLinkEn { link }.render()?),
        Locale::De => ("Anmeldung bei my-family".to_string(), MagicLinkDe { link }.render()?),
    };
    Ok((subject, body))
}

/// Render the family-invite email for `locale`.
///
/// Returns `(subject, body)`. The subject embeds the family name verbatim
/// since email clients render plain text in the subject line.
///
/// # Errors
/// Returns [`askama::Error`] if template rendering fails.
pub fn render_invite(
    locale: Locale,
    family_name: &str,
    inviter_name: &str,
    link: &str,
) -> Result<(String, String), askama::Error> {
    let (subject, body) = match locale {
        Locale::En => (
            format!("Join the {family_name} family on my-family"),
            InviteEn { link, inviter_name, family_name }.render()?,
        ),
        Locale::De => (
            format!("Einladung zur Familie {family_name} bei my-family"),
            InviteDe { link, inviter_name, family_name }.render()?,
        ),
    };
    Ok((subject, body))
}

/// Render the confirm-email-change email for `locale`.
///
/// The email is sent to the user's **current** address; `new_email` is the
/// address they want to switch to and is included in the body so the recipient
/// can verify they really initiated the change. Returns `(subject, body)`.
///
/// # Errors
/// Returns [`askama::Error`] if template rendering fails.
pub fn render_email_change(
    locale: Locale,
    link: &str,
    new_email: &str,
) -> Result<(String, String), askama::Error> {
    let (subject, body) = match locale {
        Locale::En => (
            "Confirm your email change on my-family".to_string(),
            EmailChangeEn { link, new_email }.render()?,
        ),
        Locale::De => (
            "Bestätige deine E-Mail-Änderung bei my-family".to_string(),
            EmailChangeDe { link, new_email }.render()?,
        ),
    };
    Ok((subject, body))
}

/// Render the owner-side confirmation email for an ownership transfer.
///
/// Returns `(subject, body)`. Sent to the **current** owner's address so the
/// recipient confirms they really initiated the handoff.
///
/// # Errors
/// Returns [`askama::Error`] if template rendering fails.
pub fn render_owner_transfer_owner(
    locale: Locale,
    family_name: &str,
    to_user_display_name: &str,
    link: &str,
) -> Result<(String, String), askama::Error> {
    let (subject, body) = match locale {
        Locale::En => (
            "Confirm ownership transfer".to_string(),
            OwnerTransferOwnerEn { family_name, to_user_display_name, link }.render()?,
        ),
        Locale::De => (
            "Eigentumsübertragung bestätigen".to_string(),
            OwnerTransferOwnerDe { family_name, to_user_display_name, link }.render()?,
        ),
    };
    Ok((subject, body))
}

/// Render the target-admin-side acceptance email for an ownership transfer.
///
/// Returns `(subject, body)`. Sent to the prospective new owner so they
/// confirm they accept the role swap.
///
/// # Errors
/// Returns [`askama::Error`] if template rendering fails.
pub fn render_owner_transfer_admin(
    locale: Locale,
    family_name: &str,
    from_user_display_name: &str,
    to_user_display_name: &str,
    link: &str,
) -> Result<(String, String), askama::Error> {
    let (subject, body) = match locale {
        Locale::En => (
            format!("You've been offered ownership of \"{family_name}\""),
            OwnerTransferAdminEn {
                family_name,
                from_user_display_name,
                to_user_display_name,
                link,
            }
            .render()?,
        ),
        Locale::De => (
            format!("Eigentumsübertragung für „{family_name}\" angeboten"),
            OwnerTransferAdminDe {
                family_name,
                from_user_display_name,
                to_user_display_name,
                link,
            }
            .render()?,
        ),
    };
    Ok((subject, body))
}

/// Render the daily reminder digest for `locale`.
///
/// Returns `(subject, body)`. `args.lines` are the localized event lines; the
/// subject summarizes the count + lead time. An empty `lines` slice still
/// renders, but callers should skip sending when there are no events.
///
/// # Errors
/// Returns [`askama::Error`] if template rendering fails.
pub fn render_reminder_digest(
    locale: Locale,
    args: &ReminderDigestArgs<'_>,
) -> Result<(String, String), askama::Error> {
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
    let body = match locale {
        Locale::En => ReminderDigestEn {
            lead_days: args.lead_days,
            count,
            lines: args.lines,
            tree_link: args.tree_link,
            manage_link: args.manage_link,
        }
        .render()?,
        Locale::De => ReminderDigestDe {
            lead_days: args.lead_days,
            count,
            lines: args.lines,
            tree_link: args.tree_link,
            manage_link: args.manage_link,
        }
        .render()?,
    };
    Ok((subject, body))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn renders_de_invite() {
        let (subject, body) =
            render_invite(Locale::De, "Müller", "Anna", "https://app/i/abc").unwrap();
        assert!(subject.contains("Müller"));
        assert!(body.contains("Anna"));
        assert!(body.contains("https://app/i/abc"));
    }

    #[test]
    fn renders_en_magic_link() {
        let (subject, body) = render_magic_link(Locale::En, "https://app/c/xyz").unwrap();
        assert_eq!(subject, "Sign in to my-family");
        assert!(body.contains("https://app/c/xyz"));
    }

    #[test]
    fn renders_de_magic_link_with_umlauts() {
        let (subject, body) = render_magic_link(Locale::De, "https://app/c/xyz").unwrap();
        assert_eq!(subject, "Anmeldung bei my-family");
        assert!(body.contains("gültig"));
        assert!(body.contains("https://app/c/xyz"));
    }

    #[test]
    fn renders_en_invite() {
        let (subject, body) =
            render_invite(Locale::En, "Smith", "Bob", "https://app/i/xyz").unwrap();
        assert!(subject.contains("Smith"));
        assert!(body.contains("Bob"));
        assert!(body.contains("https://app/i/xyz"));
    }

    #[test]
    fn renders_de_email_change_with_umlauts_and_new_email() {
        let (subject, body) =
            render_email_change(Locale::De, "https://app/ec/abc", "neu@example.com").unwrap();
        assert_eq!(subject, "Bestätige deine E-Mail-Änderung bei my-family");
        assert!(body.contains("neu@example.com"));
        assert!(body.contains("https://app/ec/abc"));
        assert!(body.contains("bleibt unverändert"));
    }

    #[test]
    fn renders_en_email_change() {
        let (subject, body) =
            render_email_change(Locale::En, "https://app/ec/xyz", "new@example.com").unwrap();
        assert_eq!(subject, "Confirm your email change on my-family");
        assert!(body.contains("new@example.com"));
        assert!(body.contains("https://app/ec/xyz"));
    }

    #[test]
    fn renders_de_digest_with_two_lines() {
        let lines =
            vec!["Anna — 40. Geburtstag".to_string(), "Klaus & Maria — 10. Jahrestag".to_string()];
        let (subject, body) = render_reminder_digest(
            Locale::De,
            &ReminderDigestArgs {
                lead_days: 7,
                lines: &lines,
                tree_link: "https://app/tree",
                manage_link: "https://app/account",
            },
        )
        .unwrap();
        assert!(subject.contains("In 7 Tagen"));
        assert!(subject.contains('2'));
        assert!(body.contains("Anna — 40. Geburtstag"));
        assert!(body.contains("Klaus & Maria — 10. Jahrestag"));
        assert!(body.contains("https://app/tree"));
    }

    #[test]
    fn renders_en_digest_day_of_singular() {
        let lines = vec!["Bob — 30th birthday".to_string()];
        let (subject, body) = render_reminder_digest(
            Locale::En,
            &ReminderDigestArgs {
                lead_days: 0,
                lines: &lines,
                tree_link: "https://app/tree",
                manage_link: "https://app/account",
            },
        )
        .unwrap();
        assert!(subject.contains("Today"));
        assert!(subject.contains("family date"));
        assert!(!subject.contains("dates"), "singular count uses 'date' not 'dates'");
        assert!(body.contains("Today's family dates"));
        assert!(body.contains("Bob — 30th birthday"));
    }
}
