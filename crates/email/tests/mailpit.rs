//! Sends an email through the SMTP impl against a running Mailpit and verifies
//! it landed via the Mailpit HTTP API.
//!
//! Reads two env vars (both must be set, else the test skips):
//!   EMAIL_DSN   — SMTP DSN, e.g. `smtp://mailpit:1025` (inside compose network)
//!                  or `smtp://localhost:1025` (with a host port binding).
//!   MAILPIT_API — HTTP base URL, e.g. `http://mailpit:8025` or
//!                  `http://mail.my-family.docker` (via dinghy on host) or
//!                  `http://localhost:8025`.
//!
//! Run inside the compose network with:
//!   docker run --rm --network my-family_my-family \
//!     -v "$(pwd):/workspace" -w /workspace \
//!     -e EMAIL_DSN=smtp://mailpit:1025 -e MAILPIT_API=http://mailpit:8025 \
//!     rustlang/rust:nightly cargo test -p my-family-email --test mailpit

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::print_stderr, clippy::indexing_slicing)]

use my_family_email::{EmailSender, OutboundEmail, SmtpSender};

#[tokio::test]
async fn smtp_sender_delivers_to_mailpit() {
    let Ok(dsn) = std::env::var("EMAIL_DSN") else {
        eprintln!("EMAIL_DSN not set; skipping");
        return;
    };
    let api = std::env::var("MAILPIT_API").unwrap_or_else(|_| "http://localhost:8025".to_string());

    // Drain any previous messages so the assertion is deterministic.
    reqwest::Client::new()
        .delete(format!("{api}/api/v1/messages"))
        .send()
        .await
        .expect("delete prior messages");

    let sender = SmtpSender::from_dsn(&dsn, "test", "no-reply@my-family.local", None, 5)
        .expect("build smtp sender");

    let subject = format!("test-{}", uuid::Uuid::new_v4());
    sender
        .send(OutboundEmail {
            to_addr: "recipient@example.com".into(),
            to_name: Some("Recipient".into()),
            subject: subject.clone(),
            text_body: "hello, mailpit".into(),
            html_body: None,
        })
        .await
        .expect("send");

    let resp: serde_json::Value = reqwest::Client::new()
        .get(format!("{api}/api/v1/messages"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let subjects: Vec<String> = resp["messages"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|m| m["Subject"].as_str().map(str::to_string))
        .collect();
    assert!(subjects.contains(&subject), "subject not found in Mailpit: {subjects:?}");
}
