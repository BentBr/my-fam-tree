//! Sends an email through the SMTP impl against a running Mailpit and verifies
//! it landed via the Mailpit HTTP API.
//!
//! Reads two env vars (both must be set, else the test skips):
//!   `EMAIL_DSN`   — SMTP DSN, e.g. `smtp://mailpit:1025` (inside compose network)
//!                    or `smtp://localhost:1025` (with a host port binding).
//!   `MAILPIT_API` — HTTP base URL, e.g. `http://mailpit:8025` or
//!                    `http://mail.my-fam-tree.docker` (via dinghy on host) or
//!                    `http://localhost:8025`.
//!
//! Run inside the compose network with `./scripts/cargo-in-network.sh test -p
//! my-fam-tree-email --test mailpit`, which injects both env vars pointing at the
//! compose `mailpit` service.

use my_fam_tree_email::{EmailSender, OutboundEmail, SmtpSender};

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

    let sender = SmtpSender::from_dsn(&dsn, "test", "no-reply@my-fam-tree.local", None, 5)
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

/// An HTML email (the new transactional path) must arrive as
/// `multipart/alternative` carrying both a non-empty HTML part AND the
/// inline brand logo, reachable from the HTML via `cid:logo`. This pins the
/// `multipart/related { html, logo }` wiring in [`SmtpSender`].
#[tokio::test]
async fn smtp_sender_delivers_html_with_inline_logo() {
    let Ok(dsn) = std::env::var("EMAIL_DSN") else {
        eprintln!("EMAIL_DSN not set; skipping");
        return;
    };
    let api = std::env::var("MAILPIT_API").unwrap_or_else(|_| "http://localhost:8025".to_string());
    let client = reqwest::Client::new();

    client.delete(format!("{api}/api/v1/messages")).send().await.expect("delete prior messages");

    let sender = SmtpSender::from_dsn(&dsn, "test", "no-reply@my-fam-tree.local", None, 5)
        .expect("build smtp sender");

    let subject = format!("html-{}", uuid::Uuid::new_v4());
    sender
        .send(OutboundEmail {
            to_addr: "recipient@example.com".into(),
            to_name: Some("Recipient".into()),
            subject: subject.clone(),
            text_body: "plain fallback".into(),
            html_body: Some(
                r#"<html><body><img src="cid:logo" alt="logo"/><p>hello</p></body></html>"#.into(),
            ),
        })
        .await
        .expect("send html");

    // Find our message id, then fetch the full message to inspect parts.
    let list: serde_json::Value =
        client.get(format!("{api}/api/v1/messages")).send().await.unwrap().json().await.unwrap();
    let id = list["messages"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .find(|m| m["Subject"].as_str() == Some(subject.as_str()))
        .and_then(|m| m["ID"].as_str())
        .expect("our html message present in Mailpit")
        .to_string();

    let msg: serde_json::Value = client
        .get(format!("{api}/api/v1/message/{id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // HTML alternative present and non-empty.
    assert!(
        msg["HTML"].as_str().is_some_and(|h| h.contains("hello")),
        "HTML part missing or empty: {:?}",
        msg["HTML"]
    );
    // Inline logo attachment present and addressable via cid:logo.
    let inline = msg["Inline"].as_array().cloned().unwrap_or_default();
    assert!(
        inline.iter().any(|a| a["ContentID"].as_str() == Some("logo")),
        "inline logo (ContentID=logo) not found; inline parts: {inline:?}"
    );
}
