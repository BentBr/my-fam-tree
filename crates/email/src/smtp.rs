use async_trait::async_trait;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::Tls;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use url::Url;

use crate::error::EmailError;
use crate::sender::{EmailSender, OutboundEmail};

/// Brand logo embedded inline in every HTML email (the sloth mark, a copy of
/// the FE's `apple-touch-icon`). Referenced as `cid:logo` from `_base.html`
/// and attached as a `multipart/related` inline part so it renders without
/// any externally-hosted asset (and without tripping image-blocking).
const LOGO_PNG: &[u8] = include_bytes!("../assets/logo.png");
/// Content-ID the HTML's `<img src="cid:logo">` resolves against.
const LOGO_CID: &str = "logo";

#[derive(Debug, Clone)]
pub struct SmtpSender {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from_address: String,
    from_name: String,
    reply_to: Option<String>,
}

impl SmtpSender {
    /// Build an SMTP sender from a `smtp` / `smtp+starttls` / `smtps` DSN.
    ///
    /// # Errors
    /// Returns [`EmailError::Config`] if the DSN is unparseable, has no host,
    /// uses an unsupported scheme, or the underlying `lettre` builder
    /// rejects the TLS configuration.
    pub fn from_dsn(
        dsn: &str,
        from_name: &str,
        from_address: &str,
        reply_to: Option<&str>,
        timeout_secs: u64,
    ) -> Result<Self, EmailError> {
        let url =
            Url::parse(dsn).map_err(|e| EmailError::Config(format!("invalid EMAIL_DSN: {e}")))?;
        let host =
            url.host_str().ok_or_else(|| EmailError::Config("EMAIL_DSN missing host".into()))?;
        let port = url.port().unwrap_or_else(|| match url.scheme() {
            "smtps" => 465,
            _ => 25,
        });

        let mut builder = match url.scheme() {
            "smtp" => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
                .port(port)
                .tls(Tls::None),
            "smtp+starttls" => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
                .map_err(|e| EmailError::Config(e.to_string()))?
                .port(port),
            "smtps" => AsyncSmtpTransport::<Tokio1Executor>::relay(host)
                .map_err(|e| EmailError::Config(e.to_string()))?
                .port(port),
            other => {
                return Err(EmailError::Config(format!("unsupported EMAIL_DSN scheme: {other}")));
            }
        };

        if !url.username().is_empty() {
            let user = url.username().to_owned();
            let pass = url.password().unwrap_or("").to_owned();
            builder = builder.credentials(Credentials::new(user, pass));
        }

        let transport = builder.timeout(Some(std::time::Duration::from_secs(timeout_secs))).build();

        Ok(Self {
            transport,
            from_address: from_address.to_owned(),
            from_name: from_name.to_owned(),
            reply_to: reply_to.map(str::to_owned),
        })
    }
}

#[async_trait]
impl EmailSender for SmtpSender {
    async fn send(&self, email: OutboundEmail) -> Result<(), EmailError> {
        let from: Mailbox = format!("{} <{}>", self.from_name, self.from_address)
            .parse()
            .map_err(|e: lettre::address::AddressError| EmailError::Build(e.to_string()))?;
        let to: Mailbox = match email.to_name {
            Some(n) => format!("{n} <{}>", email.to_addr).parse(),
            None => email.to_addr.parse(),
        }
        .map_err(|e: lettre::address::AddressError| EmailError::Build(e.to_string()))?;

        let mut builder = Message::builder().from(from).to(to).subject(&email.subject);
        if let Some(rt) = &self.reply_to {
            let rt_mb: Mailbox = rt
                .parse()
                .map_err(|e: lettre::address::AddressError| EmailError::Build(e.to_string()))?;
            builder = builder.reply_to(rt_mb);
        }

        let message = if let Some(html) = email.html_body {
            // multipart/alternative { text, multipart/related { html, logo } }
            // so clients pick the HTML part and resolve its `cid:logo` image
            // from the inline attachment; text-only clients fall back cleanly.
            let png =
                ContentType::parse("image/png").map_err(|e| EmailError::Build(e.to_string()))?;
            let logo = Attachment::new_inline(LOGO_CID.to_owned()).body(LOGO_PNG.to_vec(), png);
            builder.multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder().header(ContentType::TEXT_PLAIN).body(email.text_body),
                    )
                    .multipart(
                        MultiPart::related()
                            .singlepart(
                                SinglePart::builder().header(ContentType::TEXT_HTML).body(html),
                            )
                            .singlepart(logo),
                    ),
            )
        } else {
            builder.singlepart(
                SinglePart::builder().header(ContentType::TEXT_PLAIN).body(email.text_body),
            )
        }
        .map_err(|e| EmailError::Build(e.to_string()))?;

        self.transport.send(message).await.map_err(|e| EmailError::Smtp(e.to_string()))?;
        Ok(())
    }
}
