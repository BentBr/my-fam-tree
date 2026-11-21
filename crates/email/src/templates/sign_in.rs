//! Magic-link sign-in and welcome emails.

use askama::Template;

use super::{APP_NAME, Chrome, RenderedEmail, privacy_url};
use crate::locale::Locale;

// ── magic link ──────────────────────────────────────────────────────────

#[derive(Template, Debug)]
#[template(path = "magic_link_en.txt", escape = "none")]
struct MagicLinkEn<'a> {
    link: &'a str,
    name: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "magic_link_de.txt", escape = "none")]
struct MagicLinkDe<'a> {
    link: &'a str,
    name: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "magic_link_en.html")]
struct MagicLinkHtmlEn<'a> {
    chrome: Chrome<'a>,
    link: &'a str,
    name: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "magic_link_de.html")]
struct MagicLinkHtmlDe<'a> {
    chrome: Chrome<'a>,
    link: &'a str,
    name: &'a str,
}

// ── welcome ─────────────────────────────────────────────────────────────

#[derive(Template, Debug)]
#[template(path = "welcome_en.txt", escape = "none")]
struct WelcomeEn<'a> {
    link: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "welcome_de.txt", escape = "none")]
struct WelcomeDe<'a> {
    link: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "welcome_en.html")]
struct WelcomeHtmlEn<'a> {
    chrome: Chrome<'a>,
    link: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "welcome_de.html")]
struct WelcomeHtmlDe<'a> {
    chrome: Chrome<'a>,
    link: &'a str,
}

/// Render the magic-link sign-in email for `locale`.
///
/// `app_url` is the web app's public base URL (used for the header link +
/// the footer privacy link). `name` greets a known returning user by name
/// when present and non-empty; pass `None` (or an empty string) for an
/// anonymous "Hello!" greeting.
///
/// # Errors
/// Returns [`askama::Error`] if template rendering fails.
pub fn render_magic_link(
    locale: Locale,
    app_url: &str,
    link: &str,
    name: Option<&str>,
) -> Result<RenderedEmail, askama::Error> {
    let name = name.unwrap_or("");
    let privacy = privacy_url(locale, app_url);
    let chrome = Chrome::new(locale, app_url, &privacy);
    let (subject, text, html) = match locale {
        Locale::En => (
            format!("Sign in to {APP_NAME}"),
            MagicLinkEn { link, name }.render()?,
            MagicLinkHtmlEn { chrome, link, name }.render()?,
        ),
        Locale::De => (
            format!("Anmeldung bei {APP_NAME}"),
            MagicLinkDe { link, name }.render()?,
            MagicLinkHtmlDe { chrome, link, name }.render()?,
        ),
    };
    Ok(RenderedEmail { subject, text, html })
}

/// Render the welcome / onboarding email for a brand-new account.
///
/// Same single-use magic link as [`render_magic_link`] (the user still has
/// to click it to get in), but with a welcoming subject + onboarding nudge.
/// The API picks this variant only when the user row was just created.
///
/// # Errors
/// Returns [`askama::Error`] if template rendering fails.
pub fn render_welcome(
    locale: Locale,
    app_url: &str,
    link: &str,
) -> Result<RenderedEmail, askama::Error> {
    let privacy = privacy_url(locale, app_url);
    let chrome = Chrome::new(locale, app_url, &privacy);
    let (subject, text, html) = match locale {
        Locale::En => (
            format!("Welcome to {APP_NAME}"),
            WelcomeEn { link }.render()?,
            WelcomeHtmlEn { chrome, link }.render()?,
        ),
        Locale::De => (
            format!("Willkommen bei {APP_NAME}"),
            WelcomeDe { link }.render()?,
            WelcomeHtmlDe { chrome, link }.render()?,
        ),
    };
    Ok(RenderedEmail { subject, text, html })
}
