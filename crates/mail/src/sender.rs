//! The `MailSender` seam and its test/dev adapters.

use async_trait::async_trait;
use std::sync::{Arc, Mutex};

/// A composed plaintext email. `to` may carry several verified addresses for a
/// single person — SES/SMTP fan out to all of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Email {
    pub to: Vec<String>,
    pub subject: String,
    pub body_text: String,
}

/// Abstraction over the mail transport. The production adapters live in
/// `transport`; tests use [`CapturingSender`].
#[async_trait]
pub trait MailSender: Send + Sync {
    async fn send(&self, email: &Email) -> anyhow::Result<()>;
}

/// Discards everything. The default injected into the schema so tests and the
/// e2e stack never touch a real transport, and dev runs without MailHog stay
/// quiet rather than crashing.
pub struct NullSender;

#[async_trait]
impl MailSender for NullSender {
    async fn send(&self, email: &Email) -> anyhow::Result<()> {
        tracing::debug!(to = ?email.to, subject = %email.subject, "NullSender: dropping email");
        Ok(())
    }
}

/// Records every sent email in memory for assertions. Cheap to clone — clones
/// share one buffer.
#[derive(Clone, Default)]
pub struct CapturingSender {
    sent: Arc<Mutex<Vec<Email>>>,
}

impl CapturingSender {
    pub fn new() -> Self {
        Self::default()
    }
    /// A snapshot of every captured email, in send order.
    pub fn sent(&self) -> Vec<Email> {
        self.sent.lock().unwrap().clone()
    }
}

#[async_trait]
impl MailSender for CapturingSender {
    async fn send(&self, email: &Email) -> anyhow::Result<()> {
        self.sent.lock().unwrap().push(email.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn capturing_sender_records_each_send() {
        let sender = CapturingSender::new();
        let email = Email {
            to: vec!["a@dev.invalid".into()],
            subject: "hi".into(),
            body_text: "body".into(),
        };
        sender.send(&email).await.unwrap();
        sender.send(&email).await.unwrap();
        assert_eq!(sender.sent().len(), 2);
        assert_eq!(sender.sent()[0], email);
    }

    #[tokio::test]
    async fn null_sender_is_a_noop() {
        NullSender
            .send(&Email {
                to: vec![],
                subject: String::new(),
                body_text: String::new(),
            })
            .await
            .unwrap();
    }
}
