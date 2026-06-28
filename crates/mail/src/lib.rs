//! xpool mail crate — email sending + deadline-reminder selection/orchestration.
//!
//! Pure selection/window/dedup logic (`select`), bilingual templates
//! (`templates`), a `MailSender` seam with SES/SMTP/null adapters (`sender`,
//! `transport`), and the `sweep` orchestrator that ties them to
//! `storage::Repository`. The clock is always injected — logic never calls
//! `Utc::now()` itself (see `.specs/TESTING.md` §3.2).

pub mod select;
pub mod sender;
pub mod sweep;
pub mod templates;
pub mod transport;

pub use sender::{CapturingSender, Email, MailSender, NullSender};
// pub use sweep::{run_digest_sweep, run_last_call_sweep, ReminderMode, ReminderSummary};
// pub use transport::build_sender_from_env;

use chrono::{DateTime, Utc};

/// `now` for non-HTTP entrypoints (the scheduled Lambda, the xtask runner):
/// the `XPOOL_NOW` env override, else the real clock. Mirrors the HTTP clock
/// seam (`api::clock`) for a context with no request headers.
pub fn now_from_env() -> DateTime<Utc> {
    std::env::var("XPOOL_NOW")
        .ok()
        .and_then(|s| {
            DateTime::parse_from_rfc3339(s.trim())
                .ok()
                .map(|d| d.with_timezone(&Utc))
        })
        .unwrap_or_else(Utc::now)
}
