//! Pure deadline-reminder selection, windows, and dedup keys. No I/O, no clock
//! reads — `now` is always passed in (`.specs/TESTING.md` §3.2).

use chrono::{DateTime, Duration, NaiveDate, Utc};
use chrono_tz::America::Los_Angeles;
use domain::{GameId, GroupChildren, GroupId, Player, Tournament};

/// Last-call lead = trigger interval (30 min) + jitter slack (10 min) = 40 min.
/// Each 30-min tick sends for deadlines in `(now, now + 40min]`; consecutive
/// windows overlap ~10 min to absorb EventBridge jitter, and the per-(person,
/// group) dedup marker stops the overlap double-sending. The window is
/// continuous, so a deadline's minute-of-hour is irrelevant — `:30` kickoffs are
/// covered without any tick-phase alignment.
pub const LAST_CALL_LEAD: Duration = Duration::minutes(40);

/// A group/match whose deadline makes it a reminder candidate, with its
/// computed deadline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DueGroup {
    pub group_id: GroupId,
    pub deadline: DateTime<Utc>,
}

/// True when `deadline` is within the last-call window before it (and not yet passed).
/// Strict `<` at the deadline mirrors the submit gate (`API.md` Issue 27).
pub fn last_call_due(now: DateTime<Utc>, deadline: DateTime<Utc>) -> bool {
    now < deadline && deadline <= now + LAST_CALL_LEAD
}

/// The America/Los_Angeles calendar date of an instant (DST-aware via chrono-tz).
pub fn la_date(now: DateTime<Utc>) -> NaiveDate {
    now.with_timezone(&Los_Angeles).date_naive()
}

/// True when `deadline` falls on the same LA calendar day as `now`. Arg order
/// matches `last_call_due`'s `(now, deadline)` convention.
pub fn is_matchday_group(now: DateTime<Utc>, deadline: DateTime<Utc>) -> bool {
    la_date(deadline) == la_date(now)
}

/// Leaf groups (those that directly hold games) — the lockable units. Parent
/// nodes share a child's earliest kickoff, so restricting to leaves avoids
/// double-counting. Group-stage groups and one-match knockout groups are both
/// leaves, so the treatment is uniform.
fn leaf_groups(t: &Tournament) -> impl Iterator<Item = (&GroupId, DateTime<Utc>)> {
    t.groups.values().filter_map(|g| {
        if matches!(g.children, GroupChildren::Games(_)) {
            t.deadline(&g.id).map(|d| (&g.id, d))
        } else {
            None
        }
    })
}

/// Leaf groups whose deadline is within the last-call window at `now`.
pub fn groups_due_last_call(t: &Tournament, now: DateTime<Utc>) -> Vec<DueGroup> {
    let mut out: Vec<DueGroup> = leaf_groups(t)
        .filter(|(_, d)| last_call_due(now, *d))
        .map(|(id, d)| DueGroup {
            group_id: id.clone(),
            deadline: d,
        })
        .collect();
    out.sort_by(|a, b| a.group_id.cmp(&b.group_id));
    out
}

/// Leaf groups whose deadline falls on the LA calendar day of `now`.
pub fn matchday_groups(t: &Tournament, now: DateTime<Utc>) -> Vec<DueGroup> {
    let mut out: Vec<DueGroup> = leaf_groups(t)
        .filter(|(_, d)| is_matchday_group(now, *d))
        .map(|(id, d)| DueGroup {
            group_id: id.clone(),
            deadline: d,
        })
        .collect();
    out.sort_by(|a, b| a.group_id.cmp(&b.group_id));
    out
}

/// A player needs a reminder for a group when any of its games lacks a *locked*
/// prediction — i.e. missing OR unlocked (incomplete). Within a reminder window
/// `now < deadline`, so effective-lock equals the stored `locked` flag.
pub fn needs_reminder(player: &Player, game_ids: &[GameId]) -> bool {
    game_ids
        .iter()
        .any(|gid| match player.match_prediction(gid) {
            None => true,
            Some(mp) => !mp.locked,
        })
}

/// Players (globally — predictions are per-player, pools don't matter) who
/// should be reminded for a group's games: not the result user, and
/// `needs_reminder`. Returns references into the input slice, in input order.
pub fn pending_players<'a>(players: &'a [Player], game_ids: &[GameId]) -> Vec<&'a Player> {
    players
        .iter()
        .filter(|p| !p.is_result_user && needs_reminder(p, game_ids))
        .collect()
}

/// Dedup key for the hourly last-call nudge: one per (person, group). No pool —
/// predictions are per-player and global.
pub fn dedup_key_last_call(person_id: &str, group_id: &str) -> String {
    format!("{person_id}|{group_id}|1h")
}

/// Dedup key for the daily matchday digest: one per (person, LA-day). No pool.
pub fn dedup_key_digest(person_id: &str, day: NaiveDate) -> String {
    format!("{person_id}|{day}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::{
        GroupChildren, GroupGame, LockMode, MatchPrediction, Player, Round, SingleGame, TeamSlot,
        Tournament,
    };
    use std::collections::HashMap;

    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }

    fn player(id: &str, preds: Vec<MatchPrediction>) -> Player {
        Player {
            id: id.into(),
            person_id: format!("person-{id}"),
            nick: id.into(),
            full_name: id.into(),
            referrer: None,
            is_result_user: false,
            version: 0,
            match_predictions: preds,
            standings_predictions: vec![],
        }
    }

    fn pred(game_id: &str, locked: bool) -> MatchPrediction {
        MatchPrediction {
            game_id: game_id.into(),
            home_score: 1,
            away_score: 0,
            locked,
        }
    }

    // ── last_call_due: 40-min slot+slack window ───────────────────────────────
    #[test]
    fn last_call_due_only_within_the_final_window() {
        let deadline = at(2026, 6, 20, 18, 0);
        assert!(!last_call_due(at(2026, 6, 20, 17, 0), deadline)); // 60m out — outside 40m window
        assert!(!last_call_due(at(2026, 6, 20, 17, 15), deadline)); // 45m out — outside
        assert!(last_call_due(at(2026, 6, 20, 17, 25), deadline)); // 35m out — inside
        assert!(last_call_due(at(2026, 6, 20, 17, 20), deadline)); // exactly 40m out — inside
        assert!(!last_call_due(at(2026, 6, 20, 18, 0), deadline)); // at deadline (strict <)
        assert!(!last_call_due(at(2026, 6, 20, 18, 30), deadline)); // past
    }

    // ── matchday digest: LA-day match, DST-aware (June = PDT = UTC-7) ────────
    #[test]
    fn matchday_uses_la_calendar_day_not_utc() {
        // 2026-06-21 05:00 UTC == 2026-06-20 22:00 America/Los_Angeles (PDT).
        let deadline = at(2026, 6, 21, 5, 0);
        // Digest tick at LA-midnight 2026-06-20 (== 2026-06-20 07:00 UTC).
        let tick = at(2026, 6, 20, 7, 0);
        assert!(is_matchday_group(tick, deadline));
        // A deadline on the next LA day must NOT match this tick.
        let next_day = at(2026, 6, 22, 5, 0); // 2026-06-21 22:00 LA
        assert!(!is_matchday_group(tick, next_day));
    }

    #[test]
    fn la_date_of_tick() {
        // 2026-06-20 07:00 UTC is 2026-06-20 00:00 LA.
        assert_eq!(la_date(at(2026, 6, 20, 7, 0)).to_string(), "2026-06-20");
    }

    // ── needs_reminder: missing OR unlocked => true ─────────────────────────
    #[test]
    fn needs_reminder_truth_table() {
        let game_ids = vec!["M1".to_string(), "M2".to_string()];
        // both locked -> no reminder
        assert!(!needs_reminder(
            &player("a", vec![pred("M1", true), pred("M2", true)]),
            &game_ids
        ));
        // one unlocked -> reminder
        assert!(needs_reminder(
            &player("b", vec![pred("M1", true), pred("M2", false)]),
            &game_ids
        ));
        // one missing -> reminder
        assert!(needs_reminder(
            &player("c", vec![pred("M1", true)]),
            &game_ids
        ));
        // none -> reminder
        assert!(needs_reminder(&player("d", vec![]), &game_ids));
    }

    fn leaf_group(id: &str, kickoff: chrono::DateTime<Utc>) -> (Tournament, String) {
        let game = SingleGame {
            id: format!("{id}-g"),
            kickoff,
            venue: None,
            group_id: id.into(),
            home: TeamSlot {
                team_id: Some("X".into()),
                description: "x".into(),
            },
            away: TeamSlot {
                team_id: Some("Y".into()),
                description: "y".into(),
            },
            external_id: None,
        };
        let group = GroupGame {
            id: id.into(),
            name: format!("Group {id}"),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec![game.id.clone()]),
        };
        let t = Tournament {
            root: id.into(),
            groups: HashMap::from([(id.to_string(), group)]),
            games: HashMap::from([(game.id.clone(), game)]),
            teams: HashMap::new(),
        };
        (t, id.to_string())
    }

    #[test]
    fn groups_due_last_call_picks_leaf_groups_in_the_window() {
        let (t, gid) = leaf_group("A", at(2026, 6, 20, 18, 0));
        // 17:30 is 30m out — inside the 40m window
        let due = groups_due_last_call(&t, at(2026, 6, 20, 17, 30));
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].group_id, gid);
        // Outside the window -> nothing.
        assert!(groups_due_last_call(&t, at(2026, 6, 20, 12, 0)).is_empty());
    }

    #[test]
    fn matchday_groups_picks_groups_on_the_la_day() {
        let (t, gid) = leaf_group("A", at(2026, 6, 21, 5, 0)); // 2026-06-20 LA
        let due = matchday_groups(&t, at(2026, 6, 20, 7, 0)); // LA-midnight 06-20
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].group_id, gid);
        assert!(matchday_groups(&t, at(2026, 6, 21, 7, 0)).is_empty()); // wrong day
    }

    #[test]
    fn leaf_groups_excludes_parent_nodes() {
        // Two-level tree: root "R" (GroupChildren::Groups) over leaf groups
        // "A" and "B" that each hold a game. The parent's recursive deadline
        // (earliest child kickoff) would itself fall in the window, so this
        // proves parent nodes are excluded — only the leaves come back.
        fn game(id: &str, kickoff: chrono::DateTime<Utc>) -> SingleGame {
            SingleGame {
                id: format!("{id}-g"),
                kickoff,
                venue: None,
                group_id: id.into(),
                home: TeamSlot {
                    team_id: Some("X".into()),
                    description: "x".into(),
                },
                away: TeamSlot {
                    team_id: Some("Y".into()),
                    description: "y".into(),
                },
                external_id: None,
            }
        }
        fn leaf(id: &str, game_id: &str) -> GroupGame {
            GroupGame {
                id: id.into(),
                name: format!("Group {id}"),
                parent: Some("R".into()),
                round: Round::GroupStage,
                lock_mode: LockMode::LockTogether,
                carries_standings: true,
                children: GroupChildren::Games(vec![game_id.into()]),
            }
        }
        let deadline = at(2026, 6, 20, 18, 0);
        let ga = game("A", deadline);
        let gb = game("B", deadline);
        let root = GroupGame {
            id: "R".into(),
            name: "Root".into(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: false,
            children: GroupChildren::Groups(vec!["A".into(), "B".into()]),
        };
        let t = Tournament {
            root: "R".into(),
            groups: HashMap::from([
                ("R".to_string(), root),
                ("A".to_string(), leaf("A", &ga.id)),
                ("B".to_string(), leaf("B", &gb.id)),
            ]),
            games: HashMap::from([(ga.id.clone(), ga), (gb.id.clone(), gb)]),
            teams: HashMap::new(),
        };
        // Sanity: the parent node DOES have an in-window deadline, so its
        // absence below is exclusion, not a missing deadline.
        assert_eq!(t.deadline("R"), Some(deadline));

        let due = groups_due_last_call(&t, at(2026, 6, 20, 17, 30));
        let ids: Vec<&str> = due.iter().map(|g| g.group_id.as_str()).collect();
        assert_eq!(ids, vec!["A", "B"]); // only leaves, never "R"
    }

    #[test]
    fn pending_players_excludes_locked_and_result_user() {
        // Targeting is GLOBAL over all players — no pool dimension.
        let game_ids = vec!["A-g".to_string()];
        let needs = player("needs", vec![]); // no prediction -> pending
        let done = player("done", vec![pred("A-g", true)]); // locked -> not pending
        let mut ru = player("ru", vec![]);
        ru.is_result_user = true; // result user -> excluded
        let all = vec![needs, done, ru];
        let got: Vec<&str> = pending_players(&all, &game_ids)
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(got, vec!["needs"]);
    }

    #[test]
    fn dedup_keys_are_stable_and_distinct_with_no_pool() {
        // Per-player keys — pool plays no part.
        assert_eq!(dedup_key_last_call("person-x", "A"), "person-x|A|1h");
        let d = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();
        assert_eq!(dedup_key_digest("person-x", d), "person-x|2026-06-20");
        assert_ne!(
            dedup_key_last_call("person-x", "A"),
            dedup_key_digest("person-x", d)
        );
    }
}
