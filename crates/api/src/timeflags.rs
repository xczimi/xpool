//! Pure time-derived flags the API exposes so the SPA renders, not computes,
//! time-dependent state (`.specs/TESTING.md` §3.3).

use chrono::{DateTime, Duration, Utc};
use domain::Round;

/// ±2-day half-window for the "Today / Fresh" screen (UC-11).
const TODAY_WINDOW: Duration = Duration::days(2);

/// Buffer after kickoff before a result is expected: a 90-minute match needs
/// ~1h45; a knockout match may run to extra time / penalties (`API.md` §7).
pub fn result_buffer(round: Round) -> Duration {
    match round {
        Round::GroupStage => Duration::minutes(105),
        _ => Duration::minutes(150),
    }
}

/// A group's deadline has passed.
pub fn deadline_passed(deadline: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    deadline.is_some_and(|d| now > d)
}

/// A match is result-pending: its estimated end has passed and no locked
/// official result exists yet — this is what drives smart polling.
pub fn result_pending(
    kickoff: DateTime<Utc>,
    round: Round,
    has_locked_result: bool,
    now: DateTime<Utc>,
) -> bool {
    !has_locked_result && now > kickoff + result_buffer(round)
}

/// A match falls within the ±2-day Today window.
pub fn within_today_window(kickoff: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    (kickoff - now).abs() <= TODAY_WINDOW
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
        assert!(!result_pending(ko, Round::GroupStage, false, t("2026-06-20T19:00:00Z")));
        assert!(result_pending(ko, Round::GroupStage, false, t("2026-06-20T20:00:00Z")));
    }

    #[test]
    fn result_pending_false_once_a_result_is_locked() {
        let ko = t("2026-06-20T18:00:00Z");
        assert!(!result_pending(ko, Round::GroupStage, true, t("2026-06-20T23:00:00Z")));
    }

    #[test]
    fn knockout_uses_the_longer_buffer() {
        let ko = t("2026-07-10T18:00:00Z");
        // 150-min buffer -> not pending at 20:00, pending at 21:00.
        assert!(!result_pending(ko, Round::QF, false, t("2026-07-10T20:00:00Z")));
        assert!(result_pending(ko, Round::QF, false, t("2026-07-10T21:00:00Z")));
    }

    #[test]
    fn within_today_window_spans_two_days_either_side() {
        let now = t("2026-06-20T12:00:00Z");
        assert!(within_today_window(t("2026-06-21T12:00:00Z"), now));
        assert!(within_today_window(t("2026-06-19T12:00:00Z"), now));
        assert!(!within_today_window(t("2026-06-23T13:00:00Z"), now));
    }
}
