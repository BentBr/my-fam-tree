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

/// Render the magic-link sign-in email for `locale`.
///
/// Returns `(subject, body)`. Errors propagate from askama (template logic
/// errors only; the template files themselves are compiled in).
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
}
