//! Render every transactional email (EN + DE, light/dark via your OS setting)
//! to standalone HTML files you can open in a browser to eyeball the design.
//!
//! Run it (host is fine — no network needed):
//!
//! ```sh
//! cargo run -p my-fam-tree-email --example preview
//! open target/email-preview/index.html   # macOS; use xdg-open on Linux
//! ```
//!
//! Emails reference the logo as `cid:logo` (resolved from an inline MIME
//! attachment in a real send). For the browser preview we rewrite that to a
//! sibling `logo.png` and drop the file next to the HTML, so the mark renders
//! without a mail client. Toggle your OS appearance to see the dark variant —
//! the templates carry an `@media (prefers-color-scheme: dark)` block.

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    reason = "dev-only preview example: prints rendered templates, aborts on errors"
)]

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use my_fam_tree_email::{
    Locale, ReminderDigestArgs, render_email_change, render_invite, render_magic_link,
    render_owner_transfer_admin, render_owner_transfer_owner, render_reminder_digest,
    render_welcome,
};

const LOGO_PNG: &[u8] = include_bytes!("../assets/logo.png");
const APP: &str = "https://app.example.com";
const LINK: &str = "https://app.example.com/auth/consume?token=PREVIEW-TOKEN-0123456789abcdef";

fn main() {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/email-preview");
    fs::create_dir_all(&out).expect("create preview dir");
    fs::write(out.join("logo.png"), LOGO_PNG).expect("write logo");

    let digest_lines = vec![
        "Anna — 40th birthday (in 7 days)".to_owned(),
        "Klaus & Maria — 10th wedding anniversary".to_owned(),
        "Otto — 75th birthday".to_owned(),
    ];

    let mut entries: Vec<(String, String)> = Vec::new();
    for locale in [Locale::En, Locale::De] {
        let tag = match locale {
            Locale::En => "en",
            Locale::De => "de",
        };
        let mut emails = vec![
            ("welcome", render_welcome(locale, APP, LINK).unwrap()),
            ("magic_link", render_magic_link(locale, APP, LINK, None).unwrap()),
            ("magic_link_named", render_magic_link(locale, APP, LINK, Some("Anna")).unwrap()),
            ("invite", render_invite(locale, APP, "Müller", "Anna", LINK).unwrap()),
            (
                "email_change",
                render_email_change(locale, APP, LINK, "new@example.com", Some("Anna")).unwrap(),
            ),
            (
                "owner_transfer_owner",
                render_owner_transfer_owner(locale, APP, "Müller", "Bob Schmidt", LINK).unwrap(),
            ),
            (
                "owner_transfer_admin",
                render_owner_transfer_admin(
                    locale,
                    APP,
                    "Müller",
                    "Anna Müller",
                    "Bob Schmidt",
                    LINK,
                )
                .unwrap(),
            ),
        ];
        emails.push((
            "reminder_digest",
            render_reminder_digest(
                locale,
                APP,
                &ReminderDigestArgs {
                    lead_days: 7,
                    lines: &digest_lines,
                    tree_link: "https://app.example.com/tree",
                    manage_link: "https://app.example.com/account",
                },
            )
            .unwrap(),
        ));

        for (name, email) in emails {
            let file = format!("{name}_{tag}.html");
            // cid:logo only resolves inside a real MIME send; point the
            // preview at the sibling PNG instead.
            let html = email.html.replace("cid:logo", "logo.png");
            fs::write(out.join(&file), html).expect("write email html");
            entries.push((format!("{name} · {tag}"), file));
        }
    }

    // A tiny index so you can click through all of them.
    let mut index = String::from(
        "<!DOCTYPE html><meta charset=utf-8><title>Email preview</title>\
         <style>body{font:15px/1.6 system-ui;margin:40px;max-width:720px}\
         h1{font-size:20px}a{display:block;padding:6px 0}</style>\
         <h1>My Family Tree — email preview</h1>\
         <p>Toggle your OS light/dark appearance to preview both themes.</p>",
    );
    for (label, file) in &entries {
        let _ = write!(index, "<a href=\"{file}\">{label}</a>");
    }
    fs::write(out.join("index.html"), index).expect("write index");

    println!("Wrote {} previews to {}", entries.len(), out.display());
    println!("Open: {}", out.join("index.html").display());
}
