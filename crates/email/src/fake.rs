// Mutex lock failure here means another thread panicked while holding it;
// poisoning is unrecoverable so panicking is the correct behavior.
#![allow(clippy::expect_used)]

use std::sync::Mutex;

use async_trait::async_trait;

use crate::error::EmailError;
use crate::sender::{EmailSender, OutboundEmail};

#[derive(Debug, Default)]
pub struct FakeEmailSender {
    inbox: Mutex<Vec<OutboundEmail>>,
}

impl FakeEmailSender {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn drain(&self) -> Vec<OutboundEmail> {
        let mut guard = self.inbox.lock().expect("fake email mutex");
        std::mem::take(&mut *guard)
    }

    pub fn len(&self) -> usize {
        self.inbox.lock().expect("fake email mutex").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl EmailSender for FakeEmailSender {
    async fn send(&self, email: OutboundEmail) -> Result<(), EmailError> {
        self.inbox.lock().expect("fake email mutex").push(email);
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fake_captures_emails() {
        let sender = FakeEmailSender::new();
        sender
            .send(OutboundEmail {
                to_addr: "a@b.c".into(),
                to_name: None,
                subject: "hi".into(),
                text_body: "hello".into(),
                html_body: None,
            })
            .await
            .unwrap();
        let captured = sender.drain();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].subject, "hi");
    }
}
