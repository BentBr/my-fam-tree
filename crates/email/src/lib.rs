//! Outbound email: trait + SMTP implementation + in-memory fake for tests.

pub mod error;
pub mod fake;
pub mod sender;
pub mod smtp;

pub use error::EmailError;
pub use fake::FakeEmailSender;
pub use sender::{EmailSender, OutboundEmail};
pub use smtp::SmtpSender;
