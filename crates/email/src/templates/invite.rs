//! Family-invite email.

use askama::Template;

use super::{APP_NAME, Chrome, RenderedEmail, privacy_url};
use crate::locale::Locale;

// ── invite ──────────────────────────────────────────────────────────────

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
#[template(path = "invite_en.html")]
struct InviteHtmlEn<'a> {
    chrome: Chrome<'a>,
    link: &'a str,
    inviter_name: &'a str,
    family_name: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "invite_de.html")]
struct InviteHtmlDe<'a> {
    chrome: Chrome<'a>,
    link: &'a str,
    inviter_name: &'a str,
    family_name: &'a str,
}

/// Render the family-invite email for `locale`.
///
/// The subject embeds the family name verbatim. Goes to an address that may
/// not have an account yet, so there is no by-name greeting.
///
/// # Errors
/// Returns [`askama::Error`] if template rendering fails.
pub fn render_invite(
    locale: Locale,
    app_url: &str,
    family_name: &str,
    inviter_name: &str,
    link: &str,
) -> Result<RenderedEmail, askama::Error> {
    let privacy = privacy_url(locale, app_url);
    let chrome = Chrome::new(locale, app_url, &privacy);
    let (subject, text, html) = match locale {
        Locale::En => (
            format!("Join the {family_name} family on {APP_NAME}"),
            InviteEn { link, inviter_name, family_name }.render()?,
            InviteHtmlEn { chrome, link, inviter_name, family_name }.render()?,
        ),
        Locale::De => (
            format!("Einladung zur Familie {family_name} bei {APP_NAME}"),
            InviteDe { link, inviter_name, family_name }.render()?,
            InviteHtmlDe { chrome, link, inviter_name, family_name }.render()?,
        ),
    };
    Ok(RenderedEmail { subject, text, html })
}
