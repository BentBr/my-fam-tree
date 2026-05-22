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
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns and clears the captured emails.
    ///
    /// # Panics
    /// Panics if the internal `Mutex` is poisoned (only happens if another
    /// thread panicked while holding the lock).
    pub fn drain(&self) -> Vec<OutboundEmail> {
        let mut guard = self.inbox.lock().expect("fake email mutex");
        std::mem::take(&mut *guard)
    }

    /// Returns the number of currently captured emails.
    ///
    /// # Panics
    /// Panics if the internal `Mutex` is poisoned.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inbox.lock().expect("fake email mutex").len()
    }

    /// Returns `true` if no emails are currently captured.
    ///
    /// # Panics
    /// Panics if the internal `Mutex` is poisoned.
    #[must_use]
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
