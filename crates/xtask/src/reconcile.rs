//! Pure matcher for `reconcile-events`: align xpool games to TheSportsDB events
//! by unordered team-id pair + kickoff time. No I/O — the subcommand (main.rs)
//! does the fetching + writing.

use chrono::{DateTime, Utc};
use domain::Tournament;
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

/// A game projected for reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameStub {
    pub game_id: String,
    pub kickoff: DateTime<Utc>,
    pub home_team_id: Option<String>,
    pub away_team_id: Option<String>,
}

/// Fold a single (possibly accented) char to its ASCII equivalent.
/// Covers Latin diacritics used in tournament team names.
fn fold_char(c: char) -> Vec<char> {
    let s = match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' | 'Å' => "a",
        'ç' | 'Ç' => "c",
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => "e",
        'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => "i",
        'ñ' | 'Ñ' => "n",
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => "o",
        'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => "u",
        other => return vec![other],
    };
    s.chars().collect()
}

/// Fold a team name to a comparison key: lowercase, strip diacritics, keep
/// ASCII alphanumerics only. So "Curaçao" and "Curacao" both become "curacao".
fn normalize(name: &str) -> String {
    name.chars()
        .flat_map(fold_char)
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

/// Map known aliases after normalization (our spelling → SportsDB spelling, or
/// a shared canonical form). Keep this tiny and obvious.
fn alias(normalized: &str) -> &str {
    match normalized {
        "turkiye" => "turkey",
        "czechia" => "czechrepublic",
        "bosniaandherzegovina" => "bosniaherzegovina",
        other => other,
    }
}

fn team_key(name: &str) -> String {
    alias(&normalize(name)).to_string()
}

/// Resolve our team ids → SportsDB idTeam. Prefers a committed external_id;
/// else matches by normalized+aliased name.
/// `our_teams` is `(our_team_id, our_name, committed_external_id)`.
pub fn resolve_team_ids(
    our_teams: &[(String, String, Option<String>)],
    rows: &[TeamRow],
) -> (HashMap<String, String>, Vec<String>) {
    let by_key: HashMap<String, String> = rows
        .iter()
        .map(|r| (team_key(&r.str_team), r.id_team.clone()))
        .collect();
    let mut resolved = HashMap::new();
    let mut unresolved = Vec::new();
    for (id, name, ext) in our_teams {
        if let Some(e) = ext {
            resolved.insert(id.clone(), e.clone());
        } else if let Some(idt) = by_key.get(&team_key(name)) {
            resolved.insert(id.clone(), idt.clone());
        } else {
            unresolved.push(id.clone());
        }
    }
    (resolved, unresolved)
}

type PairIndex<'a> = HashMap<(String, String), Vec<(&'a Event, Option<DateTime<Utc>>)>>;

/// Match games to events by unordered team-id pair + kickoff (within 2-day tolerance).
/// `team_external_id` maps our team id → SportsDB idTeam.
pub fn reconcile(
    games: &[GameStub],
    team_external_id: &HashMap<String, String>,
    events: &[Event],
) -> Report {
    fn event_kickoff(e: &Event) -> Option<DateTime<Utc>> {
        if let Some(ts) = &e.str_timestamp {
            if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
                return Some(dt.with_timezone(&Utc));
            }
        }
        chrono::NaiveDate::parse_from_str(&e.date_event, "%Y-%m-%d")
            .ok()
            .map(|d| {
                DateTime::<Utc>::from_naive_utc_and_offset(d.and_hms_opt(0, 0, 0).unwrap(), Utc)
            })
    }

    fn pair(a: &str, b: &str) -> (String, String) {
        if a <= b {
            (a.to_string(), b.to_string())
        } else {
            (b.to_string(), a.to_string())
        }
    }

    // Index events by unordered idTeam pair.
    let mut by_pair: PairIndex<'_> = HashMap::new();
    for e in events {
        if e.id_home_team.is_empty() || e.id_away_team.is_empty() {
            continue;
        }
        by_pair
            .entry(pair(&e.id_home_team, &e.id_away_team))
            .or_default()
            .push((e, event_kickoff(e)));
    }

    const TOLERANCE: chrono::Duration = chrono::Duration::days(2);
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
        let hit = resolved
            .and_then(|(h, a)| by_pair.get(&pair(h, a)))
            .and_then(|cands| {
                cands
                    .iter()
                    .filter(|(_, k)| k.is_none_or(|k| (k - g.kickoff).abs() <= TOLERANCE))
                    .min_by_key(|(_, k)| {
                        k.map_or(i64::MAX, |k| (k - g.kickoff).num_seconds().abs())
                    })
            });
        match hit {
            Some((e, _)) => report.matched.push(Match {
                game_id: g.game_id.clone(),
                id_event: e.id_event.clone(),
            }),
            None => report.unmatched_games.push(g.game_id.clone()),
        }
    }
    report
}

/// Set each matched game's `external_id` to its proposed `idEvent` (immutable:
/// returns a new tournament, leaving `t` untouched). Only games whose stored
/// `external_id` differs from the proposal are changed — every other field,
/// notably resolved knockout `team_id`s written by the post-result recompute,
/// is preserved. So `reconcile-events --apply` is safe to run mid-tournament
/// (no destructive re-import) and idempotent. Returns the new tournament and
/// how many games changed.
pub fn apply_external_ids(t: &Tournament, matched: &[Match]) -> (Tournament, usize) {
    let mut next = t.clone();
    let mut changed = 0usize;
    for m in matched {
        if let Some(game) = next.games.get_mut(&m.game_id) {
            if game.external_id.as_deref() != Some(m.id_event.as_str()) {
                game.external_id = Some(m.id_event.clone());
                changed += 1;
            }
        }
    }
    (next, changed)
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
            str_timestamp: None,
        }
    }

    fn kickoff(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn matches_by_team_ids_and_kickoff() {
        let events = vec![ev("2461106", "2026-06-15", "133", "999")];
        let team_ext: HashMap<String, String> = [
            ("SWE".to_string(), "133".to_string()),
            ("TUN".to_string(), "999".to_string()),
        ]
        .into_iter()
        .collect();
        let games = vec![GameStub {
            game_id: "M5".into(),
            kickoff: kickoff("2026-06-15T02:00:00+00:00"),
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
        let team_ext = HashMap::new();
        let games = vec![GameStub {
            game_id: "M5".into(),
            kickoff: kickoff("2026-06-15T02:00:00+00:00"),
            home_team_id: Some("SWE".into()),
            away_team_id: Some("TUN".into()),
        }];
        let report = reconcile(&games, &team_ext, &events);
        assert!(report.matched.is_empty());
        assert_eq!(report.unmatched_games, vec!["M5".to_string()]);
    }

    #[test]
    fn reports_unmatched_when_no_event_matches() {
        let events = vec![ev("2461106", "2026-06-15", "133", "999")];
        let team_ext: HashMap<String, String> = [
            ("SWE".to_string(), "133".to_string()),
            ("TUN".to_string(), "999".to_string()),
        ]
        .into_iter()
        .collect();
        // kickoff 20 days later — outside the 2-day tolerance
        let games = vec![GameStub {
            game_id: "M9".into(),
            kickoff: kickoff("2026-07-05T02:00:00+00:00"),
            home_team_id: Some("SWE".into()),
            away_team_id: Some("TUN".into()),
        }];
        let report = reconcile(&games, &team_ext, &events);
        assert!(report.matched.is_empty());
        assert_eq!(report.unmatched_games, vec!["M9".to_string()]);
    }

    // resolve_team_ids tests

    fn row(id: &str, name: &str) -> TeamRow {
        TeamRow {
            id_team: id.into(),
            str_team: name.into(),
        }
    }

    #[test]
    fn resolves_by_normalized_name_curacao() {
        // SportsDB has "Curaçao"; our tournament has "Curacao"
        let rows = vec![row("555", "Curaçao")];
        let our = vec![("CUR".to_string(), "Curacao".to_string(), None)];
        let (resolved, unresolved) = resolve_team_ids(&our, &rows);
        assert_eq!(resolved.get("CUR"), Some(&"555".to_string()));
        assert!(unresolved.is_empty());
    }

    #[test]
    fn resolves_by_alias_czechia() {
        // SportsDB has "Czech Republic"; we have "Czechia"
        let rows = vec![row("200", "Czech Republic")];
        let our = vec![("CZE".to_string(), "Czechia".to_string(), None)];
        let (resolved, unresolved) = resolve_team_ids(&our, &rows);
        assert_eq!(resolved.get("CZE"), Some(&"200".to_string()));
        assert!(unresolved.is_empty());
    }

    #[test]
    fn returns_unresolved_when_no_match() {
        let rows = vec![row("133", "Sweden")];
        let our = vec![("XYZ".to_string(), "Atlantis".to_string(), None)];
        let (resolved, unresolved) = resolve_team_ids(&our, &rows);
        assert!(resolved.get("XYZ").is_none());
        assert_eq!(unresolved, vec!["XYZ".to_string()]);
    }

    #[test]
    fn prefers_committed_external_id_over_name() {
        // Even if name matches, committed external_id wins
        let rows = vec![row("133", "Sweden")];
        let our = vec![(
            "SWE".to_string(),
            "Sweden".to_string(),
            Some("999".to_string()),
        )];
        let (resolved, _) = resolve_team_ids(&our, &rows);
        assert_eq!(resolved.get("SWE"), Some(&"999".to_string()));
    }

    // apply_external_ids tests

    use domain::{SingleGame, TeamSlot};

    /// A knockout game with resolved teams (as the post-result recompute leaves
    /// it) and the given external_id.
    fn ko_game(id: &str, ext: Option<&str>) -> SingleGame {
        SingleGame {
            id: id.into(),
            kickoff: "2026-06-28T19:00:00Z".parse().unwrap(),
            venue: None,
            group_id: format!("KO-{id}"),
            home: TeamSlot {
                team_id: Some("ARG".into()),
                description: "2A".into(),
            },
            away: TeamSlot {
                team_id: Some("BRA".into()),
                description: "2B".into(),
            },
            external_id: ext.map(|s| s.into()),
        }
    }

    fn tournament_with(games: Vec<SingleGame>) -> Tournament {
        Tournament {
            root: "root".into(),
            groups: HashMap::new(),
            games: games.into_iter().map(|g| (g.id.clone(), g)).collect(),
            teams: HashMap::new(),
        }
    }

    #[test]
    fn apply_sets_external_id_and_preserves_resolved_teams() {
        let t = tournament_with(vec![ko_game("M73", None)]);
        let matched = vec![Match {
            game_id: "M73".into(),
            id_event: "2499618".into(),
        }];

        let (next, changed) = apply_external_ids(&t, &matched);

        assert_eq!(changed, 1);
        let g = &next.games["M73"];
        assert_eq!(g.external_id, Some("2499618".to_string()));
        // The resolved knockout teams must survive untouched.
        assert_eq!(g.home.team_id, Some("ARG".to_string()));
        assert_eq!(g.away.team_id, Some("BRA".to_string()));
        // Input tournament is not mutated (immutability).
        assert_eq!(t.games["M73"].external_id, None);
    }

    #[test]
    fn apply_is_idempotent_noop_when_already_set() {
        let t = tournament_with(vec![ko_game("M73", Some("2499618"))]);
        let matched = vec![Match {
            game_id: "M73".into(),
            id_event: "2499618".into(),
        }];

        let (next, changed) = apply_external_ids(&t, &matched);

        assert_eq!(changed, 0, "re-applying the same id must be a no-op");
        assert_eq!(next.games["M73"].external_id, Some("2499618".to_string()));
    }

    #[test]
    fn apply_overwrites_a_changed_id_and_ignores_unknown_games() {
        let t = tournament_with(vec![ko_game("M73", Some("old"))]);
        let matched = vec![
            Match {
                game_id: "M73".into(),
                id_event: "2499618".into(),
            },
            // A game id not present in the tournament is silently skipped.
            Match {
                game_id: "M999".into(),
                id_event: "1".into(),
            },
        ];

        let (next, changed) = apply_external_ids(&t, &matched);

        assert_eq!(changed, 1);
        assert_eq!(next.games["M73"].external_id, Some("2499618".to_string()));
        assert!(!next.games.contains_key("M999"));
    }
}
