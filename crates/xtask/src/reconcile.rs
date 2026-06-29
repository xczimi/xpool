//! Pure matcher for `reconcile-events`: align xpool games to TheSportsDB events
//! by unordered team-id pair + kickoff time. No I/O — the subcommand (main.rs)
//! does the fetching + writing.

use chrono::{DateTime, Utc};
use domain::Tournament;
use sportsdb::{Event, TeamRow};
use std::collections::HashMap;

/// One proposed mapping row: our game aligned to a SportsDB event, carrying the
/// event's real kickoff so `--apply` can correct broadcast-shifted times.
#[derive(Debug, PartialEq, Eq)]
pub struct Match {
    pub game_id: String,
    pub id_event: String,
    /// The matched event's kickoff (from `strTimestamp`/`dateEvent`), if known.
    pub event_kickoff: Option<DateTime<Utc>>,
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
            Some((e, k)) => report.matched.push(Match {
                game_id: g.game_id.clone(),
                id_event: e.id_event.clone(),
                event_kickoff: *k,
            }),
            None => report.unmatched_games.push(g.game_id.clone()),
        }
    }
    report
}

/// One corrected kickoff: a knockout game whose stored time was wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KickoffChange {
    pub game_id: String,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

/// What `apply_matches` changed.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ApplyReport {
    pub external_id_changed: usize,
    pub kickoff_changes: Vec<KickoffChange>,
}

/// Align each matched game to its SportsDB event (immutable: returns a new
/// tournament, leaving `t` untouched):
///
/// - **`external_id`** is set on every matched game whose stored value differs
///   (group games already carry theirs, so it is a no-op for them).
/// - **`kickoff`** is corrected from the event's real timestamp, but **only for
///   knockout games** — broadcast scheduling shifts knockout kickoffs after the
///   bracket is drawn, and a wrong stored kickoff breaks both the live-score
///   window and the per-match prediction lock. Group-stage times are already
///   played out, so they are left untouched to keep the blast radius small.
///
/// Every other field — notably the resolved knockout `team_id`s the post-result
/// recompute writes — is preserved, so this is safe to run mid-tournament and is
/// idempotent. Returns the new tournament and a report of what changed.
pub fn apply_matches(t: &Tournament, matched: &[Match]) -> (Tournament, ApplyReport) {
    let mut next = t.clone();
    let mut report = ApplyReport::default();
    for m in matched {
        // Read knockout-ness from the untouched input (groups never change).
        let is_knockout = t
            .games
            .get(&m.game_id)
            .and_then(|g| t.groups.get(&g.group_id))
            .is_some_and(|grp| grp.round != domain::Round::GroupStage);

        if let Some(game) = next.games.get_mut(&m.game_id) {
            if game.external_id.as_deref() != Some(m.id_event.as_str()) {
                game.external_id = Some(m.id_event.clone());
                report.external_id_changed += 1;
            }
            if is_knockout {
                if let Some(real) = m.event_kickoff {
                    if game.kickoff != real {
                        report.kickoff_changes.push(KickoffChange {
                            game_id: m.game_id.clone(),
                            from: game.kickoff,
                            to: real,
                        });
                        game.kickoff = real;
                    }
                }
            }
        }
    }
    (next, report)
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
                id_event: "2461106".into(),
                // No str_timestamp on the event → kickoff falls back to dateEvent midnight.
                event_kickoff: Some(kickoff("2026-06-15T00:00:00+00:00")),
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

    // apply_matches tests

    use domain::{GroupChildren, GroupGame, LockMode, Round, SingleGame, TeamSlot};

    /// A game in `group_id` with resolved teams (as the post-result recompute
    /// leaves a knockout game), kicking off `2026-06-28T19:00:00Z`.
    fn game_in(id: &str, group_id: &str, ext: Option<&str>) -> SingleGame {
        SingleGame {
            id: id.into(),
            kickoff: kickoff("2026-06-28T19:00:00+00:00"),
            venue: None,
            group_id: group_id.into(),
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

    fn one_game_group(id: &str, round: Round, game_id: &str) -> GroupGame {
        GroupGame {
            id: id.into(),
            name: id.into(),
            parent: Some("root".into()),
            round,
            lock_mode: LockMode::LockPerMatch,
            carries_standings: false,
            children: GroupChildren::Games(vec![game_id.into()]),
        }
    }

    /// A one-game tournament whose single game sits in a group of the given round.
    fn tournament_of(round: Round, game: SingleGame) -> Tournament {
        let grp = one_game_group(&game.group_id.clone(), round, &game.id.clone());
        Tournament {
            root: "root".into(),
            groups: HashMap::from([(grp.id.clone(), grp)]),
            games: HashMap::from([(game.id.clone(), game)]),
            teams: HashMap::new(),
        }
    }

    fn match_of(game_id: &str, id_event: &str, event_kickoff: Option<DateTime<Utc>>) -> Match {
        Match {
            game_id: game_id.into(),
            id_event: id_event.into(),
            event_kickoff,
        }
    }

    #[test]
    fn apply_sets_external_id_and_preserves_resolved_teams() {
        let t = tournament_of(Round::R32, game_in("M73", "KO-M73", None));
        // event_kickoff None → isolate the external_id behaviour.
        let matched = vec![match_of("M73", "2499618", None)];

        let (next, report) = apply_matches(&t, &matched);

        assert_eq!(report.external_id_changed, 1);
        assert!(report.kickoff_changes.is_empty());
        let g = &next.games["M73"];
        assert_eq!(g.external_id, Some("2499618".to_string()));
        // The resolved knockout teams must survive untouched.
        assert_eq!(g.home.team_id, Some("ARG".to_string()));
        assert_eq!(g.away.team_id, Some("BRA".to_string()));
        // Input tournament is not mutated (immutability).
        assert_eq!(t.games["M73"].external_id, None);
    }

    #[test]
    fn apply_is_idempotent_noop_when_already_aligned() {
        let g = game_in("M73", "KO-M73", Some("2499618"));
        let stored_kickoff = g.kickoff;
        let t = tournament_of(Round::R32, g);
        // Same id, same kickoff → nothing to change.
        let matched = vec![match_of("M73", "2499618", Some(stored_kickoff))];

        let (next, report) = apply_matches(&t, &matched);

        assert_eq!(report.external_id_changed, 0, "same id must be a no-op");
        assert!(
            report.kickoff_changes.is_empty(),
            "same kickoff must be a no-op"
        );
        assert_eq!(next.games["M73"].external_id, Some("2499618".to_string()));
    }

    #[test]
    fn apply_overwrites_a_changed_id_and_ignores_unknown_games() {
        let t = tournament_of(Round::R32, game_in("M73", "KO-M73", Some("old")));
        let matched = vec![
            match_of("M73", "2499618", None),
            // A game id not present in the tournament is silently skipped.
            match_of("M999", "1", None),
        ];

        let (next, report) = apply_matches(&t, &matched);

        assert_eq!(report.external_id_changed, 1);
        assert_eq!(next.games["M73"].external_id, Some("2499618".to_string()));
        assert!(!next.games.contains_key("M999"));
    }

    #[test]
    fn apply_corrects_a_broadcast_shifted_knockout_kickoff() {
        let t = tournament_of(Round::R32, game_in("M76", "KO-M76", Some("2499835")));
        // Real kickoff is 8h earlier than what we stored (the broadcast shift).
        let real = kickoff("2026-06-29T17:00:00+00:00");
        let matched = vec![match_of("M76", "2499835", Some(real))];

        let (next, report) = apply_matches(&t, &matched);

        assert_eq!(report.external_id_changed, 0, "id already correct");
        assert_eq!(report.kickoff_changes.len(), 1);
        let change = &report.kickoff_changes[0];
        assert_eq!(change.game_id, "M76");
        assert_eq!(change.from, kickoff("2026-06-28T19:00:00+00:00"));
        assert_eq!(change.to, real);
        assert_eq!(next.games["M76"].kickoff, real);
        // Immutability: the input keeps its original kickoff.
        assert_eq!(t.games["M76"].kickoff, kickoff("2026-06-28T19:00:00+00:00"));
    }

    #[test]
    fn apply_leaves_group_stage_kickoffs_untouched() {
        // A group-stage game whose stored kickoff differs from the event's — the
        // sync must NOT touch it (group stage is already played out).
        let t = tournament_of(Round::GroupStage, game_in("M1", "A", Some("2391728")));
        let other = kickoff("2026-06-11T22:00:00+00:00");
        let matched = vec![match_of("M1", "2391728", Some(other))];

        let (next, report) = apply_matches(&t, &matched);

        assert!(
            report.kickoff_changes.is_empty(),
            "group-stage kickoff must not be synced"
        );
        assert_eq!(
            next.games["M1"].kickoff,
            kickoff("2026-06-28T19:00:00+00:00"),
            "group-stage kickoff unchanged"
        );
    }
}
