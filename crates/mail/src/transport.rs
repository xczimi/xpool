//! Concrete `MailSender` adapters and env-based selection.
//!
//! - [`SesSender`] (prod): `aws-sdk-sesv2`, reusing the Lambda role's
//!   `ses:SendEmail` grant — no new credentials.
//! - [`SmtpSender`] (local): `lettre` SMTP, pointed at MailHog (`:1025`).
//! - Selection mirrors `api`'s `ReportedResultSource` env-pick.

use crate::sender::{Email, MailSender};
use anyhow::Context as _;
use async_trait::async_trait;
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message as SesMessage};
use lettre::{message::Mailbox, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::sync::Arc;

/// Which transport to construct.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportKind {
    Smtp,
    Ses,
    Null,
}

/// Pure selector: explicit `MAIL_TRANSPORT` wins; otherwise a local DynamoDB
/// endpoint implies local SMTP (MailHog), else SES.
pub fn choose_transport(
    mail_transport: Option<&str>,
    dynamo_endpoint: Option<&str>,
) -> TransportKind {
    match mail_transport.map(str::trim) {
        Some("ses") => TransportKind::Ses,
        Some("smtp") => TransportKind::Smtp,
        Some("null") => TransportKind::Null,
        _ => {
            let local = dynamo_endpoint
                .map(str::trim)
                .is_some_and(|e| !e.is_empty());
            if local {
                TransportKind::Smtp
            } else {
                TransportKind::Ses
            }
        }
    }
}

/// The verified `From:` address. SES requires it to be on the verified domain
/// (`var.ses_domain`, `xczimi.com`). Default: `pool@xczimi.com` — the address
/// Auth0 already sends from (established, monitored, DKIM/SPF-aligned).
fn from_address() -> String {
    std::env::var("MAIL_FROM").unwrap_or_else(|_| "pool@xczimi.com".to_owned())
}

/// The `Reply-To` address. Returns `MAIL_REPLY_TO` if set and non-empty,
/// otherwise falls back to [`from_address`]. This lets replies reach a human
/// inbox (the "reply to opt out" flow) and can be repointed without a code
/// change.
fn reply_to_address() -> String {
    std::env::var("MAIL_REPLY_TO")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(from_address)
}

/// `lettre` SMTP sender (local MailHog by default; plaintext, no TLS).
pub struct SmtpSender {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

impl SmtpSender {
    pub fn from_env() -> anyhow::Result<Self> {
        let host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_owned());
        let port: u16 = std::env::var("SMTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(1025);
        // builder_dangerous = plaintext (MailHog speaks no TLS).
        let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host)
            .port(port)
            .build();
        Ok(Self {
            transport,
            from: from_address(),
        })
    }
}

#[async_trait]
impl MailSender for SmtpSender {
    async fn send(&self, email: &Email) -> anyhow::Result<()> {
        let from: Mailbox = self.from.parse().context("parsing MAIL_FROM")?;
        let reply_to: Mailbox = reply_to_address()
            .parse()
            .context("parsing MAIL_REPLY_TO / MAIL_FROM as reply-to")?;
        let mut builder = Message::builder()
            .from(from)
            .reply_to(reply_to)
            .subject(&email.subject);
        for addr in &email.to {
            let mbox: Mailbox = addr
                .parse()
                .with_context(|| format!("parsing to-address {addr}"))?;
            builder = builder.to(mbox);
        }
        let msg = builder
            .body(email.body_text.clone())
            .context("building SMTP message")?;
        self.transport.send(msg).await.context("SMTP send")?;
        Ok(())
    }
}

/// `aws-sdk-sesv2` sender (prod). Uses the ambient AWS credentials/role.
pub struct SesSender {
    client: aws_sdk_sesv2::Client,
    from: String,
}

impl SesSender {
    pub async fn from_env() -> anyhow::Result<Self> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        Ok(Self {
            client: aws_sdk_sesv2::Client::new(&config),
            from: from_address(),
        })
    }
}

#[async_trait]
impl MailSender for SesSender {
    async fn send(&self, email: &Email) -> anyhow::Result<()> {
        let reply_to = reply_to_address();
        let dest = Destination::builder()
            .set_to_addresses(Some(email.to.clone()))
            .build();
        let subject = Content::builder()
            .data(&email.subject)
            .build()
            .context("SES subject content")?;
        let text = Content::builder()
            .data(&email.body_text)
            .build()
            .context("SES body content")?;
        let body = Body::builder().text(text).build();
        let msg = SesMessage::builder().subject(subject).body(body).build();
        let content = EmailContent::builder().simple(msg).build();
        self.client
            .send_email()
            .from_email_address(&self.from)
            .set_reply_to_addresses(Some(vec![reply_to]))
            .destination(dest)
            .content(content)
            .send()
            .await
            .context("SES send_email")?;
        Ok(())
    }
}

/// Build the sender chosen by the environment.
pub async fn build_sender_from_env() -> anyhow::Result<Arc<dyn MailSender>> {
    let mail_transport = std::env::var("MAIL_TRANSPORT").ok();
    let dynamo_endpoint = std::env::var("DYNAMO_ENDPOINT").ok();
    match choose_transport(mail_transport.as_deref(), dynamo_endpoint.as_deref()) {
        TransportKind::Smtp => Ok(Arc::new(SmtpSender::from_env()?)),
        TransportKind::Ses => Ok(Arc::new(SesSender::from_env().await?)),
        TransportKind::Null => Ok(Arc::new(crate::sender::NullSender)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_transport_wins() {
        assert_eq!(
            choose_transport(Some("ses"), Some("http://localhost:8000")),
            TransportKind::Ses
        );
        assert_eq!(choose_transport(Some("smtp"), None), TransportKind::Smtp);
        assert_eq!(choose_transport(Some("null"), None), TransportKind::Null);
    }

    #[test]
    fn local_dynamo_endpoint_defaults_to_smtp() {
        assert_eq!(
            choose_transport(None, Some("http://localhost:8000")),
            TransportKind::Smtp
        );
    }

    #[test]
    fn no_hints_defaults_to_ses() {
        assert_eq!(choose_transport(None, None), TransportKind::Ses);
        assert_eq!(choose_transport(None, Some("")), TransportKind::Ses);
    }
}
