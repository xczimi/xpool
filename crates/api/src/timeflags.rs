//! Pure time-derived flags the API exposes so the SPA renders, not computes,
//! time-dependent state (`.specs/TESTING.md` §3.3).

use chrono::{DateTime, Duration, Utc};
use domain::Round;

/// ±2-day half-window for the "Today / Fresh" screen (UC-11).
const TODAY_WINDOW: Duration = Duration::days(2);

/// Buffer after kickoff before a result is expected: regulation plus stoppage
/// (~1h45). We deliberately treat a match as "resulted" at the end of 90' and do
/// NOT wait out extra time / penalties — that in-between state is not modelled,
/// and a longer knockout buffer would otherwise delay a genuinely-entered
/// knockout result from materialising the bracket (`API.md` §7). The `round` is
/// kept in the signature so a per-round buffer can return later if needed.
pub fn result_buffer(_round: Round) -> Duration {
    Duration::minutes(105)
}

/// A group's deadline has passed.
pub fn deadline_passed(deadline: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    deadline.is_some_and(|d| now > d)
}

/// A match is result-pending: its estimated end has passed and no official
/// result has been entered yet — this is what drives smart polling.
pub fn result_pending(
    kickoff: DateTime<Utc>,
    round: Round,
    has_result: bool,
    now: DateTime<Utc>,
) -> bool {
    !has_result && now > kickoff + result_buffer(round)
}

/// A match falls within the ±2-day Today window.
pub fn within_today_window(kickoff: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    (kickoff - now).abs() <= TODAY_WINDOW
}

/// A match kicks off on the current (UTC) calendar day. The Today screen shows a
/// ±2-day window; this narrower flag lets the SPA highlight the matches that are
/// *actually* today. UTC-day equality keeps it server-authoritative and consistent
/// with the `XPOOL_NOW` / `X-Dev-Now` clock seam.
pub fn is_today(kickoff: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    kickoff.date_naive() == now.date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn deadline_passed_is_false_before_and_true_after() {
        let d = Some(t("2026-06-20T12:00:00Z"));
        assert!(!deadline_passed(d, t("2026-06-20T11:00:00Z")));
        assert!(deadline_passed(d, t("2026-06-20T13:00:00Z")));
        assert!(!deadline_passed(None, t("2026-06-20T13:00:00Z")));
    }

    #[test]
    fn result_pending_true_after_buffer_when_no_result() {
        let ko = t("2026-06-20T18:00:00Z");
        // Group buffer = 105 min -> pending at 20:00, not at 19:00.
        assert!(!result_pending(
            ko,
            Round::GroupStage,
            false,
            t("2026-06-20T19:00:00Z")
        ));
        assert!(result_pending(
            ko,
            Round::GroupStage,
            false,
            t("2026-06-20T20:00:00Z")
        ));
    }

    #[test]
    fn result_pending_false_once_a_result_is_entered() {
        let ko = t("2026-06-20T18:00:00Z");
        assert!(!result_pending(
            ko,
            Round::GroupStage,
            true,
            t("2026-06-20T23:00:00Z")
        ));
    }

    #[test]
    fn knockout_uses_the_same_regulation_buffer() {
        let ko = t("2026-07-10T18:00:00Z");
        // 105-min buffer for every round (extra time / penalties not modelled):
        // not pending at +60m (19:00), pending at +120m (20:00).
        assert!(!result_pending(
            ko,
            Round::QF,
            false,
            t("2026-07-10T19:00:00Z")
        ));
        assert!(result_pending(
            ko,
            Round::QF,
            false,
            t("2026-07-10T20:00:00Z")
        ));
    }

    #[test]
    fn within_today_window_spans_two_days_either_side() {
        let now = t("2026-06-20T12:00:00Z");
        assert!(within_today_window(t("2026-06-21T12:00:00Z"), now));
        assert!(within_today_window(t("2026-06-19T12:00:00Z"), now));
        assert!(!within_today_window(t("2026-06-23T13:00:00Z"), now));
    }

    #[test]
    fn is_today_is_same_utc_calendar_day() {
        let now = t("2026-06-20T12:00:00Z");
        // Same UTC day, any time within it.
        assert!(is_today(t("2026-06-20T00:30:00Z"), now));
        assert!(is_today(t("2026-06-20T22:00:00Z"), now));
        // Adjacent days are within the ±2-day window but are NOT today.
        assert!(!is_today(t("2026-06-19T23:59:00Z"), now));
        assert!(!is_today(t("2026-06-21T00:01:00Z"), now));
    }
}
