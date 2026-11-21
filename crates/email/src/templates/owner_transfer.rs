//! Ownership-transfer emails (owner side + incoming admin side).

use askama::Template;

use super::{Chrome, RenderedEmail, privacy_url};
use crate::locale::Locale;

// ── owner transfer (owner side) ─────────────────────────────────────────

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
#[template(path = "owner_transfer_owner_en.html")]
struct OwnerTransferOwnerHtmlEn<'a> {
    chrome: Chrome<'a>,
    family_name: &'a str,
    to_user_display_name: &'a str,
    link: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "owner_transfer_owner_de.html")]
struct OwnerTransferOwnerHtmlDe<'a> {
    chrome: Chrome<'a>,
    family_name: &'a str,
    to_user_display_name: &'a str,
    link: &'a str,
}

// ── owner transfer (incoming admin side) ────────────────────────────────

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
#[template(path = "owner_transfer_admin_en.html")]
struct OwnerTransferAdminHtmlEn<'a> {
    chrome: Chrome<'a>,
    family_name: &'a str,
    from_user_display_name: &'a str,
    to_user_display_name: &'a str,
    link: &'a str,
}

#[derive(Template, Debug)]
#[template(path = "owner_transfer_admin_de.html")]
struct OwnerTransferAdminHtmlDe<'a> {
    chrome: Chrome<'a>,
    family_name: &'a str,
    from_user_display_name: &'a str,
    to_user_display_name: &'a str,
    link: &'a str,
}

/// Render the owner-side confirmation email for an ownership transfer.
///
/// Sent to the **current** owner's address so they confirm they really
/// initiated the handoff.
///
/// # Errors
/// Returns [`askama::Error`] if template rendering fails.
pub fn render_owner_transfer_owner(
    locale: Locale,
    app_url: &str,
    family_name: &str,
    to_user_display_name: &str,
    link: &str,
) -> Result<RenderedEmail, askama::Error> {
    let privacy = privacy_url(locale, app_url);
    let chrome = Chrome::new(locale, app_url, &privacy);
    let (subject, text, html) = match locale {
        Locale::En => (
            "Confirm ownership transfer".to_owned(),
            OwnerTransferOwnerEn { family_name, to_user_display_name, link }.render()?,
            OwnerTransferOwnerHtmlEn { chrome, family_name, to_user_display_name, link }
                .render()?,
        ),
        Locale::De => (
            "Eigentumsübertragung bestätigen".to_owned(),
            OwnerTransferOwnerDe { family_name, to_user_display_name, link }.render()?,
            OwnerTransferOwnerHtmlDe { chrome, family_name, to_user_display_name, link }
                .render()?,
        ),
    };
    Ok(RenderedEmail { subject, text, html })
}

/// Render the target-admin-side acceptance email for an ownership transfer.
///
/// Sent to the prospective new owner so they confirm they accept the swap.
///
/// # Errors
/// Returns [`askama::Error`] if template rendering fails.
pub fn render_owner_transfer_admin(
    locale: Locale,
    app_url: &str,
    family_name: &str,
    from_user_display_name: &str,
    to_user_display_name: &str,
    link: &str,
) -> Result<RenderedEmail, askama::Error> {
    let privacy = privacy_url(locale, app_url);
    let chrome = Chrome::new(locale, app_url, &privacy);
    let (subject, text, html) = match locale {
        Locale::En => (
            format!("You've been offered ownership of \"{family_name}\""),
            OwnerTransferAdminEn {
                family_name,
                from_user_display_name,
                to_user_display_name,
                link,
            }
            .render()?,
            OwnerTransferAdminHtmlEn {
                chrome,
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
            OwnerTransferAdminHtmlDe {
                chrome,
                family_name,
                from_user_display_name,
                to_user_display_name,
                link,
            }
            .render()?,
        ),
    };
    Ok(RenderedEmail { subject, text, html })
}
