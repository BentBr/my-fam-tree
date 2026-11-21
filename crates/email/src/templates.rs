//! Locale-aware rendering of transactional emails (plain-text + HTML).
//!
//! Templates live in `crates/email/templates/` (askama's default lookup dir)
//! and compile into the binary. Each email has a `.txt` part and a `.html`
//! part; the HTML parts share `_base.html` (the bulletproof, table-based
//! Stack Overflow short-transactional shell, restyled to our CI with a
//! `prefers-color-scheme` dark variant) via `{% extends %}` and pull common
//! pieces — button, paste-link, footnote — from `_macros.html`.
//!
//! Escaping: the `.txt` templates set `escape = "none"` (plain text needs no
//! escaping); the `.html` templates use askama's default HTML escaper, so
//! every interpolated user value (names, family names, email addresses,
//! event lines) is HTML-escaped — the URLs we emit are trusted but escaping
//! them in `href` attributes is harmless.
//!
//! Each `render_*` helper returns a [`RenderedEmail`] (`subject` + `text` +
//! `html`) so call-sites can populate both MIME alternatives of an
//! [`crate::sender::OutboundEmail`].

use crate::locale::Locale;

/// Product name shown in every email (header wordmark, footer, subjects).
pub const APP_NAME: &str = "My Family Tree";

/// A fully rendered email: subject + both MIME alternatives.
///
/// `text` is the plain-text part; `html` is the styled
/// multipart-alternative part. Call-sites hand `text` to
/// [`crate::sender::OutboundEmail::text_body`] and `html` to `html_body`.
#[derive(Debug, Clone)]
pub struct RenderedEmail {
    pub subject: String,
    pub text: String,
    pub html: String,
}

/// Shared chrome (header / footer / dark-mode) context for every HTML email.
///
/// Held as a nested field on each HTML template struct so the `_base.html`
/// layout can reference `chrome.*` regardless of which concrete email is
/// extending it.
#[derive(Debug)]
struct Chrome<'a> {
    lang: &'a str,
    app_name: &'a str,
    app_url: &'a str,
    privacy_url: &'a str,
    privacy_label: &'a str,
}

impl<'a> Chrome<'a> {
    const fn new(locale: Locale, app_url: &'a str, privacy_url: &'a str) -> Self {
        let (lang, privacy_label) = match locale {
            Locale::En => ("en", "Privacy"),
            Locale::De => ("de", "Datenschutz"),
        };
        Self { lang, app_name: APP_NAME, app_url, privacy_url, privacy_label }
    }
}

/// Build the privacy-policy URL from the web app's public base URL.
///
/// The route is locale-prefixed `/{lang}/data-policy` (see the FE router), so
/// the footer link in every email lands on the recipient's-language legal
/// page rather than bouncing through a redirect.
fn privacy_url(locale: Locale, app_url: &str) -> String {
    locale.link(app_url, "/data-policy")
}

mod digest;
mod email_change;
mod invite;
mod owner_transfer;
mod sign_in;

pub use digest::{ReminderDigestArgs, render_reminder_digest};
pub use email_change::render_email_change;
pub use invite::render_invite;
pub use owner_transfer::{render_owner_transfer_admin, render_owner_transfer_owner};
pub use sign_in::{render_magic_link, render_welcome};

#[cfg(test)]
mod tests {
    use super::*;

    const APP: &str = "https://app";

    #[test]
    fn renders_de_invite() {
        let email = render_invite(Locale::De, APP, "Müller", "Anna", "https://app/i/abc").unwrap();
        assert!(email.subject.contains("Müller"));
        assert!(email.text.contains("Anna"));
        assert!(email.text.contains("https://app/i/abc"));
        assert!(email.html.contains("Anna"));
        assert!(email.html.contains("cid:logo"));
    }

    #[test]
    fn renders_en_magic_link() {
        let email = render_magic_link(Locale::En, APP, "https://app/c/xyz", None).unwrap();
        assert_eq!(email.subject, "Sign in to My Family Tree");
        assert!(email.text.contains("https://app/c/xyz"));
        assert!(email.html.contains("https://app/c/xyz"));
        // No name → anonymous greeting, no "Hello <name>,".
        assert!(email.text.contains("Hello!"));
    }

    #[test]
    fn footer_privacy_link_is_locale_prefixed() {
        // The footer's privacy link must carry the recipient's locale so it
        // lands on the right-language legal page (no redirect hop).
        let en = render_magic_link(Locale::En, APP, "https://app/en/c/xyz", None).unwrap();
        assert!(en.html.contains("https://app/en/data-policy"));
        let de = render_magic_link(Locale::De, APP, "https://app/de/c/xyz", None).unwrap();
        assert!(de.html.contains("https://app/de/data-policy"));
    }

    #[test]
    fn renders_magic_link_greets_known_user_by_name() {
        let email = render_magic_link(Locale::En, APP, "https://app/c/xyz", Some("Anna")).unwrap();
        assert!(email.text.contains("Hello Anna,"));
        assert!(email.html.contains("Hello Anna,"));
        assert!(!email.text.contains("Hello!"));
    }

    #[test]
    fn html_escapes_user_supplied_names() {
        // A family name with HTML must not break out into markup.
        let email =
            render_invite(Locale::En, APP, "<script>x</script>", "Bob", "https://app/i/xyz")
                .unwrap();
        assert!(!email.html.contains("<script>x</script>"));
        assert!(email.html.contains("&#60;script&#62;") || email.html.contains("&lt;script&gt;"));
    }

    #[test]
    fn renders_de_magic_link_with_umlauts() {
        let email = render_magic_link(Locale::De, APP, "https://app/c/xyz", None).unwrap();
        assert_eq!(email.subject, "Anmeldung bei My Family Tree");
        assert!(email.text.contains("gültig"));
        assert!(email.text.contains("https://app/c/xyz"));
    }

    #[test]
    fn renders_en_welcome_with_signin_link() {
        let email = render_welcome(Locale::En, APP, "https://app/c/new").unwrap();
        assert_eq!(email.subject, "Welcome to My Family Tree");
        assert!(email.text.to_lowercase().contains("welcome"));
        assert!(email.text.contains("https://app/c/new"));
        assert!(email.html.contains("https://app/c/new"));
        assert!(email.html.contains("cid:logo"));
        assert!(email.html.contains("My Family Tree"));
        // Privacy link is locale-prefixed (EN here).
        assert!(email.html.contains("https://app/en/data-policy"));
    }

    #[test]
    fn renders_de_welcome_with_umlauts_and_signin_link() {
        let email = render_welcome(Locale::De, "https://app/", "https://app/c/new").unwrap();
        assert_eq!(email.subject, "Willkommen bei My Family Tree");
        assert!(email.text.contains("gültig"));
        assert!(email.html.contains("https://app/c/new"));
        // Locale-prefixed (DE) privacy link; a trailing slash on the base URL
        // must not double up.
        assert!(email.html.contains("https://app/de/data-policy"));
        assert!(!email.html.contains("https://app//de/data-policy"));
    }

    #[test]
    fn renders_en_invite() {
        let email = render_invite(Locale::En, APP, "Smith", "Bob", "https://app/i/xyz").unwrap();
        assert!(email.subject.contains("Smith"));
        assert!(email.text.contains("Bob"));
        assert!(email.html.contains("https://app/i/xyz"));
    }

    #[test]
    fn renders_de_email_change_with_umlauts_and_new_email() {
        let email =
            render_email_change(Locale::De, APP, "https://app/ec/abc", "neu@example.com", None)
                .unwrap();
        assert_eq!(email.subject, "Bestätige deine E-Mail-Änderung bei My Family Tree");
        assert!(email.text.contains("neu@example.com"));
        assert!(email.text.contains("https://app/ec/abc"));
        assert!(email.text.contains("bleibt unverändert"));
        assert!(email.html.contains("neu@example.com"));
    }

    #[test]
    fn renders_en_email_change() {
        let email =
            render_email_change(Locale::En, APP, "https://app/ec/xyz", "new@example.com", None)
                .unwrap();
        assert_eq!(email.subject, "Confirm your email change on My Family Tree");
        assert!(email.text.contains("new@example.com"));
        assert!(email.html.contains("https://app/ec/xyz"));
    }

    #[test]
    fn renders_owner_transfer_owner_and_admin() {
        let owner =
            render_owner_transfer_owner(Locale::En, APP, "Müller", "Bob", "https://app/ot/owner")
                .unwrap();
        assert_eq!(owner.subject, "Confirm ownership transfer");
        assert!(owner.html.contains("Müller"));
        assert!(owner.html.contains("Bob"));
        assert!(owner.html.contains("https://app/ot/owner"));

        let admin = render_owner_transfer_admin(
            Locale::De,
            APP,
            "Müller",
            "Alice",
            "Bob",
            "https://app/ot/admin",
        )
        .unwrap();
        assert!(admin.subject.contains("Müller"));
        assert!(admin.html.contains("Alice"));
        assert!(admin.html.contains("Bob"));
    }

    #[test]
    fn renders_de_digest_with_two_lines() {
        let lines =
            vec!["Anna — 40. Geburtstag".to_owned(), "Klaus & Maria — 10. Jahrestag".to_owned()];
        let email = render_reminder_digest(
            Locale::De,
            APP,
            &ReminderDigestArgs {
                lead_days: 7,
                lines: &lines,
                tree_link: "https://app/tree",
                manage_link: "https://app/account",
            },
        )
        .unwrap();
        assert!(email.subject.contains("In 7 Tagen"));
        assert!(email.subject.contains('2'));
        assert!(email.text.contains("Anna — 40. Geburtstag"));
        assert!(email.text.contains("Klaus & Maria — 10. Jahrestag"));
        assert!(email.html.contains("https://app/tree"));
        // `&` in an event line must be HTML-escaped in the HTML part.
        assert!(
            email.html.contains("Klaus &#38; Maria") || email.html.contains("Klaus &amp; Maria")
        );
    }

    #[test]
    fn renders_en_digest_day_of_singular() {
        let lines = vec!["Bob — 30th birthday".to_owned()];
        let email = render_reminder_digest(
            Locale::En,
            APP,
            &ReminderDigestArgs {
                lead_days: 0,
                lines: &lines,
                tree_link: "https://app/tree",
                manage_link: "https://app/account",
            },
        )
        .unwrap();
        assert!(email.subject.contains("Today"));
        assert!(email.subject.contains("family date"));
        assert!(!email.subject.contains("dates"), "singular count uses 'date' not 'dates'");
        assert!(email.text.contains("Today's family dates"));
        assert!(email.text.contains("Bob — 30th birthday"));
    }
}
