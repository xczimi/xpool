//! Scheduled deadline-reminder Lambda (the reminder heartbeat).
//!
//! Built only under `--features lambda` (see `required-features` in Cargo.toml).
//! Two EventBridge schedules invoke it with a constant payload selecting the
//! mode: `{"mode":"last_call"}` (every 30 min) or `{"mode":"digest"}` (daily, at
//! 00:00 America/Los_Angeles). Both call the shared `mail` sweep orchestrator.
//!
//! The clock honours `XPOOL_NOW` (so the path is testable on dev) then the real
//! clock — the same precedence as the HTTP clock seam, minus request headers.

use lambda_runtime::{service_fn, LambdaEvent};
use mail::ReminderMode;
use serde_json::Value;
use std::sync::Arc;
use storage::{DynamoRepository, Repository};

async fn handler(event: LambdaEvent<Value>) -> Result<Value, lambda_runtime::Error> {
    let mode_str = event
        .payload
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("last_call");
    let mode = ReminderMode::parse(mode_str)
        .ok_or_else(|| lambda_runtime::Error::from(format!("unknown reminder mode: {mode_str}")))?;

    let repo = DynamoRepository::from_env()
        .await
        .map_err(|e| lambda_runtime::Error::from(e.to_string()))?;
    let repo: Arc<dyn Repository> = Arc::new(repo);
    let mail = mail::build_sender_from_env()
        .await
        .map_err(|e| lambda_runtime::Error::from(e.to_string()))?;
    let now = mail::now_from_env();

    let summary = match mode {
        ReminderMode::LastCall => {
            mail::run_last_call_sweep(repo.as_ref(), mail.as_ref(), now).await
        }
        ReminderMode::Digest => mail::run_digest_sweep(repo.as_ref(), mail.as_ref(), now).await,
    }
    .map_err(|e| lambda_runtime::Error::from(e.to_string()))?;

    tracing::info!(
        mode = ?mode,
        recipients = summary.recipients,
        sent = summary.sent,
        skipped_no_email = summary.skipped_no_email,
        deduped = summary.deduped,
        "reminder sweep complete"
    );
    Ok(serde_json::json!({
        "mode": mode_str,
        "recipients": summary.recipients,
        "sent": summary.sent,
        "skipped_no_email": summary.skipped_no_email,
        "deduped": summary.deduped,
    }))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .json()
        .init();
    lambda_runtime::run(service_fn(handler)).await
}
