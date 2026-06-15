//! The field subset of a TheSportsDB event that xpool consumes.

/// A match event as reported by TheSportsDB. Scores are `None` until played.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub id_event: String,
    pub date_event: String,
    pub id_home_team: String,
    pub id_away_team: String,
    pub int_home_score: Option<i64>,
    pub int_away_score: Option<i64>,
    pub str_status: String,
}

impl Event {
    /// A match TheSportsDB considers played out (final score available).
    /// `strStatus` is free-form upstream; treat the documented finished
    /// markers as final and require both scores present.
    pub fn is_finished(&self) -> bool {
        let s = self.str_status.as_str();
        let finished = matches!(s, "Match Finished" | "FT" | "AET" | "Finished")
            || s.eq_ignore_ascii_case("ft");
        finished && self.int_home_score.is_some() && self.int_away_score.is_some()
    }
}
