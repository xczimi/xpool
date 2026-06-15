//! Pure matcher for `reconcile-events`: align xpool games to TheSportsDB events
//! by (date, home team, away team) and propose `M# → idEvent` mappings for
//! human review. No I/O — the subcommand (main.rs) does the fetching + writing.

use sportsdb::{Event, TeamRow};
use std::collections::HashMap;

/// One proposed mapping row.
#[derive(Debug, PartialEq, Eq)]
pub struct Match {
    pub game_id: String,
    pub id_event: String,
}

/// The outcome of a reconcile pass — matches plus games we could not align.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub matched: Vec<Match>,
    pub unmatched_games: Vec<String>,
}

/// Match games to events. `team_external_id` maps our team id → SportsDB idTeam
/// (from a prior team reconcile or the committed `external_id`s). A game matches
/// an event when the kickoff date (UTC `YYYY-MM-DD`) and both team ids agree.
pub fn reconcile(
    games: &[(String, String, Option<String>, Option<String>)], // (game_id, date, home_team_id, away_team_id)
    team_external_id: &HashMap<String, String>,
    events: &[Event],
) -> Report {
    // Index events by (date, home idTeam, away idTeam).
    let mut by_key: HashMap<(String, String, String), &Event> = HashMap::new();
    for e in events {
        by_key.insert(
            (
                e.date_event.clone(),
                e.id_home_team.clone(),
                e.id_away_team.clone(),
            ),
            e,
        );
    }

    let mut report = Report::default();
    for (game_id, date, home, away) in games {
        let resolved = home
            .as_ref()
            .and_then(|h| team_external_id.get(h))
            .zip(away.as_ref().and_then(|a| team_external_id.get(a)));
        let hit = resolved.and_then(|(h, a)| by_key.get(&(date.clone(), h.clone(), a.clone())));
        match hit {
            Some(e) => report.matched.push(Match {
                game_id: game_id.clone(),
                id_event: e.id_event.clone(),
            }),
            None => report.unmatched_games.push(game_id.clone()),
        }
    }
    report
}

/// Resolve `idTeam` for every SportsDB team name we can match by exact name —
/// a helper for first-time team reconcile (case-insensitive exact match).
pub fn team_ids_by_name(rows: &[TeamRow]) -> HashMap<String, String> {
    rows.iter()
        .map(|r| (r.str_team.to_lowercase(), r.id_team.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(id: &str, date: &str, h: &str, a: &str) -> Event {
        Event {
            id_event: id.into(),
            date_event: date.into(),
            id_home_team: h.into(),
            id_away_team: a.into(),
            int_home_score: None,
            int_away_score: None,
            str_status: "Not Started".into(),
        }
    }

    #[test]
    fn matches_by_date_and_team_ids() {
        let events = vec![ev("2461106", "2026-06-15", "133", "999")];
        let team_ext: HashMap<String, String> = [
            ("SWE".to_string(), "133".to_string()),
            ("TUN".to_string(), "999".to_string()),
        ]
        .into_iter()
        .collect();
        let games = vec![(
            "M5".to_string(),
            "2026-06-15".to_string(),
            Some("SWE".to_string()),
            Some("TUN".to_string()),
        )];
        let report = reconcile(&games, &team_ext, &events);
        assert_eq!(
            report.matched,
            vec![Match {
                game_id: "M5".into(),
                id_event: "2461106".into()
            }]
        );
        assert!(report.unmatched_games.is_empty());
    }

    #[test]
    fn reports_unmatched_when_team_id_missing() {
        let events = vec![ev("2461106", "2026-06-15", "133", "999")];
        let team_ext = HashMap::new(); // no team mapping yet
        let games = vec![(
            "M5".to_string(),
            "2026-06-15".to_string(),
            Some("SWE".to_string()),
            Some("TUN".to_string()),
        )];
        let report = reconcile(&games, &team_ext, &events);
        assert!(report.matched.is_empty());
        assert_eq!(report.unmatched_games, vec!["M5".to_string()]);
    }
}
