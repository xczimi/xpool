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

/// A game projected for reconciliation. `date` MUST be formatted `YYYY-MM-DD`
/// (UTC) to match `Event::date_event` from TheSportsDB, or the key match fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameStub {
    pub game_id: String,
    pub date: String,
    pub home_team_id: Option<String>,
    pub away_team_id: Option<String>,
}

/// Match games to events. `team_external_id` maps our team id → SportsDB idTeam
/// (from a prior team reconcile or the committed `external_id`s). A game matches
/// an event when the kickoff date (UTC `YYYY-MM-DD`) and both team ids agree.
pub fn reconcile(
    games: &[GameStub],
    team_external_id: &HashMap<String, String>,
    events: &[Event],
) -> Report {
    // Index events by (date, home idTeam, away idTeam).
    let mut by_key: HashMap<(String, String, String), &Event> = HashMap::new();
    for e in events {
        if let Some(prev) = by_key.insert(
            (
                e.date_event.clone(),
                e.id_home_team.clone(),
                e.id_away_team.clone(),
            ),
            e,
        ) {
            eprintln!(
                "WARN: duplicate SportsDB event key ({} {} v {}): {} overwritten by {}",
                e.date_event, e.id_home_team, e.id_away_team, prev.id_event, e.id_event
            );
        }
    }

    let mut report = Report::default();
    for g in games {
        let resolved = g
            .home_team_id
            .as_ref()
            .and_then(|h| team_external_id.get(h))
            .zip(
                g.away_team_id
                    .as_ref()
                    .and_then(|a| team_external_id.get(a)),
            );
        let hit = resolved.and_then(|(h, a)| by_key.get(&(g.date.clone(), h.clone(), a.clone())));
        match hit {
            Some(e) => report.matched.push(Match {
                game_id: g.game_id.clone(),
                id_event: e.id_event.clone(),
            }),
            None => report.unmatched_games.push(g.game_id.clone()),
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
        let games = vec![GameStub {
            game_id: "M5".into(),
            date: "2026-06-15".into(),
            home_team_id: Some("SWE".into()),
            away_team_id: Some("TUN".into()),
        }];
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
        let games = vec![GameStub {
            game_id: "M5".into(),
            date: "2026-06-15".into(),
            home_team_id: Some("SWE".into()),
            away_team_id: Some("TUN".into()),
        }];
        let report = reconcile(&games, &team_ext, &events);
        assert!(report.matched.is_empty());
        assert_eq!(report.unmatched_games, vec!["M5".to_string()]);
    }

    #[test]
    fn reports_unmatched_when_no_event_matches() {
        // Team ids resolve, but no event exists for that (date, home, away).
        let events = vec![ev("2461106", "2026-06-15", "133", "999")];
        let team_ext: HashMap<String, String> = [
            ("SWE".to_string(), "133".to_string()),
            ("TUN".to_string(), "999".to_string()),
        ]
        .into_iter()
        .collect();
        let games = vec![GameStub {
            game_id: "M9".into(),
            date: "2026-06-20".into(), // different date -> no match
            home_team_id: Some("SWE".into()),
            away_team_id: Some("TUN".into()),
        }];
        let report = reconcile(&games, &team_ext, &events);
        assert!(report.matched.is_empty());
        assert_eq!(report.unmatched_games, vec!["M9".to_string()]);
    }
}
