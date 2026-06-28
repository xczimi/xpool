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

/// Pure clock resolver: an optional RFC3339 override string, else `real_now`.
/// Extracted so `now_from_env` is testable without mutating process env
/// (mirrors `api::clock`).
fn now_from(env_val: Option<&str>, real_now: DateTime<Utc>) -> DateTime<Utc> {
    env_val
        .and_then(|s| DateTime::parse_from_rfc3339(s.trim()).ok())
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or(real_now)
}

/// `now` for non-HTTP entrypoints (the scheduled Lambda, the xtask runner):
/// the `XPOOL_NOW` env override, else the real clock. Mirrors the HTTP clock
/// seam (`api::clock`) for a context with no request headers.
pub fn now_from_env() -> DateTime<Utc> {
    now_from(std::env::var("XPOOL_NOW").ok().as_deref(), Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 20, 12, 0, 0).unwrap()
    }

    #[test]
    fn valid_override_is_parsed() {
        let got = now_from(Some(" 2026-06-21T18:30:00Z "), fixed_now());
        assert_eq!(got, Utc.with_ymd_and_hms(2026, 6, 21, 18, 30, 0).unwrap());
    }

    #[test]
    fn none_falls_back_to_real_now() {
        assert_eq!(now_from(None, fixed_now()), fixed_now());
    }

    #[test]
    fn malformed_value_falls_back_to_real_now() {
        assert_eq!(now_from(Some("not-a-date"), fixed_now()), fixed_now());
    }
}
