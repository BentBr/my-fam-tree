//! Confirm-email-change email.

use askama::Template;

use super::{APP_NAME, Chrome, RenderedEmail, privacy_url};
use crate::locale::Locale;

// ── email change ────────────────────────────────────────────────────────

#[derive(Template, Debug)]
#[template(path = "email_change_en.txt", escape = "none")]
struct EmailChangeEn<'a> {
    link: &'a str,
    new_email: &'a str,
    name: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "email_change_de.txt", escape = "none")]
struct EmailChangeDe<'a> {
    link: &'a str,
    new_email: &'a str,
    name: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "email_change_en.html")]
struct EmailChangeHtmlEn<'a> {
    chrome: Chrome<'a>,
    link: &'a str,
    new_email: &'a str,
    name: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "email_change_de.html")]
struct EmailChangeHtmlDe<'a> {
    chrome: Chrome<'a>,
    link: &'a str,
    new_email: &'a str,
    name: &'a str,
}

/// Render the confirm-email-change email for `locale`.
///
/// Sent to the user's **current** address; `new_email` is the address they
/// want to switch to. `name` greets the (known) account holder by name.
///
/// # Errors
/// Returns [`askama::Error`] if template rendering fails.
pub fn render_email_change(
    locale: Locale,
    app_url: &str,
    link: &str,
    new_email: &str,
    name: Option<&str>,
) -> Result<RenderedEmail, askama::Error> {
    let name = name.unwrap_or("");
    let privacy = privacy_url(locale, app_url);
    let chrome = Chrome::new(locale, app_url, &privacy);
    let (subject, text, html) = match locale {
        Locale::En => (
            format!("Confirm your email change on {APP_NAME}"),
            EmailChangeEn { link, new_email, name }.render()?,
            EmailChangeHtmlEn { chrome, link, new_email, name }.render()?,
        ),
        Locale::De => (
            format!("Bestätige deine E-Mail-Änderung bei {APP_NAME}"),
            EmailChangeDe { link, new_email, name }.render()?,
            EmailChangeHtmlDe { chrome, link, new_email, name }.render()?,
        ),
    };
    Ok(RenderedEmail { subject, text, html })
}
