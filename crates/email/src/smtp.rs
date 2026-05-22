use async_trait::async_trait;
use lettre::message::header::ContentType;
use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::Tls;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use url::Url;

use crate::error::EmailError;
use crate::sender::{EmailSender, OutboundEmail};

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
            let user = url.username().to_string();
            let pass = url.password().unwrap_or("").to_string();
            builder = builder.credentials(Credentials::new(user, pass));
        }

        let transport = builder.timeout(Some(std::time::Duration::from_secs(timeout_secs))).build();

        Ok(Self {
            transport,
            from_address: from_address.to_string(),
            from_name: from_name.to_string(),
            reply_to: reply_to.map(str::to_string),
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
            builder.multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder().header(ContentType::TEXT_PLAIN).body(email.text_body),
                    )
                    .singlepart(SinglePart::builder().header(ContentType::TEXT_HTML).body(html)),
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
