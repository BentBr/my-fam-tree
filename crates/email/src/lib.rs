//! Outbound email: trait + SMTP impl + Fake + locale-aware templates.

pub mod error;
pub mod fake;
pub mod locale;
pub mod sender;
pub mod smtp;
pub mod templates;

pub use error::EmailError;
pub use fake::FakeEmailSender;
pub use locale::Locale;
pub use sender::{EmailSender, OutboundEmail};
pub use smtp::SmtpSender;
pub use templates::{
    ReminderDigestArgs, render_email_change, render_invite, render_magic_link,
    render_owner_transfer_admin, render_owner_transfer_owner, render_reminder_digest,
};
