//! Pure decoders for TheSportsDB V2 JSON envelopes. The V2 API keys its
//! top-level array by *operation* (`schedule`/`livescore`/`list`), not entity
//! (`.specs/THESPORTSDB_API.md` §6). These functions take the raw body and
//! return the field subset xpool uses — no HTTP, fully unit-testable.

use crate::model::{Event, TeamRow};
use serde::Deserialize;

#[derive(Deserialize)]
struct ScheduleEnvelope {
    schedule: Option<Vec<RawEvent>>,
}

#[derive(Deserialize)]
struct LivescoreEnvelope {
    livescore: Option<Vec<RawEvent>>,
}

#[derive(Deserialize)]
struct RawEvent {
    #[serde(rename = "idEvent")]
    id_event: Option<String>,
    #[serde(rename = "dateEvent")]
    date_event: Option<String>,
    #[serde(rename = "idHomeTeam")]
    id_home_team: Option<String>,
    #[serde(rename = "idAwayTeam")]
    id_away_team: Option<String>,
    #[serde(rename = "intHomeScore")]
    int_home_score: Option<serde_json::Value>,
    #[serde(rename = "intAwayScore")]
    int_away_score: Option<serde_json::Value>,
    #[serde(rename = "strStatus")]
    str_status: Option<String>,
    #[serde(rename = "strTimestamp")]
    str_timestamp: Option<String>,
}

/// Scores arrive as a string ("2"), a number (2), or null. Normalise to i64.
fn score(v: &Option<serde_json::Value>) -> Option<i64> {
    match v {
        Some(serde_json::Value::Number(n)) => n.as_i64(),
        Some(serde_json::Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

impl RawEvent {
    fn into_event(self) -> Option<Event> {
        Some(Event {
            id_event: self.id_event?,
            date_event: self.date_event.unwrap_or_default(),
            id_home_team: self.id_home_team.unwrap_or_default(),
            id_away_team: self.id_away_team.unwrap_or_default(),
            int_home_score: score(&self.int_home_score),
            int_away_score: score(&self.int_away_score),
            str_status: self.str_status.unwrap_or_default(),
            str_timestamp: self.str_timestamp,
        })
    }
}

/// Decode a `/schedule/league/...` body into events.
pub fn decode_schedule(body: &str) -> anyhow::Result<Vec<Event>> {
    let env: ScheduleEnvelope = serde_json::from_str(body)?;
    Ok(env
        .schedule
        .unwrap_or_default()
        .into_iter()
        .filter_map(RawEvent::into_event)
        .collect())
}

/// Decode a `/livescore/...` body into events.
pub fn decode_livescore(body: &str) -> anyhow::Result<Vec<Event>> {
    let env: LivescoreEnvelope = serde_json::from_str(body)?;
    Ok(env
        .livescore
        .unwrap_or_default()
        .into_iter()
        .filter_map(RawEvent::into_event)
        .collect())
}

#[derive(Deserialize)]
struct ListEnvelope {
    list: Option<Vec<RawTeam>>,
}

#[derive(Deserialize)]
struct RawTeam {
    #[serde(rename = "idTeam")]
    id_team: Option<String>,
    #[serde(rename = "strTeam")]
    str_team: Option<String>,
}

/// Decode a `/list/teams/{leagueId}` body into team rows.
pub fn decode_teams(body: &str) -> anyhow::Result<Vec<TeamRow>> {
    let env: ListEnvelope = serde_json::from_str(body)?;
    Ok(env
        .list
        .unwrap_or_default()
        .into_iter()
        .filter_map(|t| {
            Some(TeamRow {
                id_team: t.id_team?,
                str_team: t.str_team.unwrap_or_default(),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_schedule_with_string_and_numeric_scores() {
        let body = r#"{"schedule":[
            {"idEvent":"2461106","dateEvent":"2026-06-15","idHomeTeam":"H","idAwayTeam":"A","intHomeScore":"2","intAwayScore":"1","strStatus":"Match Finished"},
            {"idEvent":"2461112","dateEvent":"2026-06-20","idHomeTeam":"H2","idAwayTeam":"A2","intHomeScore":null,"intAwayScore":null,"strStatus":"Not Started"}
        ]}"#;
        let events = decode_schedule(body).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].id_event, "2461106");
        assert_eq!(events[0].int_home_score, Some(2));
        assert_eq!(events[0].int_away_score, Some(1));
        assert_eq!(events[0].str_status, "Match Finished");
        assert_eq!(events[1].int_home_score, None);
    }

    #[test]
    fn decodes_null_schedule_as_empty() {
        assert_eq!(decode_schedule(r#"{"schedule":null}"#).unwrap().len(), 0);
    }

    #[test]
    fn decodes_livescore_envelope() {
        let body = r#"{"livescore":[{"idEvent":"2461106","intHomeScore":2,"intAwayScore":1,"strStatus":"HT"}]}"#;
        let events = decode_livescore(body).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].str_status, "HT");
    }

    #[test]
    fn decodes_teams_list() {
        let body = r#"{"list":[{"idTeam":"133","strTeam":"Sweden"}]}"#;
        let teams = decode_teams(body).unwrap();
        assert_eq!(teams.len(), 1);
        assert_eq!(teams[0].id_team, "133");
        assert_eq!(teams[0].str_team, "Sweden");
    }

    #[test]
    fn decodes_str_timestamp() {
        let body = r#"{"schedule":[{"idEvent":"1","dateEvent":"2026-06-15","strTimestamp":"2026-06-15T02:00:00+00:00"}]}"#;
        let events = decode_schedule(body).unwrap();
        assert_eq!(
            events[0].str_timestamp,
            Some("2026-06-15T02:00:00+00:00".to_string())
        );
    }
}
