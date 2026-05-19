use async_trait::async_trait;

use crate::error::EmailError;

#[derive(Debug, Clone)]
pub struct OutboundEmail {
    pub to_addr: String,
    pub to_name: Option<String>,
    pub subject: String,
    pub text_body: String,
    pub html_body: Option<String>,
}

#[async_trait]
pub trait EmailSender: Send + Sync + std::fmt::Debug {
    async fn send(&self, email: OutboundEmail) -> Result<(), EmailError>;
}
