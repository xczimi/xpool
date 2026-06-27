//! Tests for third_place_ranking (FWC26_RULES.md §3) — the display ranking.

use domain::*;
use fwc26::third_place_ranking;
use std::collections::HashMap;

fn kickoff() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2026-06-11T18:00:00Z")
        .unwrap()
        .into()
}

fn team(id: &str) -> Team {
    Team {
        id: id.to_string(),
        name: id.to_string(),
        short_code: id.to_string(),
        flag: None,
        external_id: None,
    }
}

fn slot_team(team_id: &str, desc: &str) -> TeamSlot {
    TeamSlot {
        team_id: Some(team_id.to_string()),
        description: desc.to_string(),
    }
}

fn slot_placeholder(desc: &str) -> TeamSlot {
    TeamSlot {
        team_id: None,
        description: desc.to_string(),
    }
}

fn game(id: &str, group_id: &str, home: TeamSlot, away: TeamSlot) -> SingleGame {
    SingleGame {
        id: id.to_string(),
        kickoff: kickoff(),
        venue: None,
        group_id: group_id.to_string(),
        home,
        away,
        external_id: None,
    }
}

fn group_stage_group(id: &str, games: Vec<String>) -> GroupGame {
    GroupGame {
        id: id.to_string(),
        name: format!("Group {}", id.to_uppercase().replace("GROUP-", "")),
        parent: Some("root".to_string()),
        round: Round::GroupStage,
        lock_mode: LockMode::LockTogether,
        carries_standings: true,
        children: GroupChildren::Games(games),
    }
}

fn knockout_group(id: &str, game_ids: Vec<String>) -> GroupGame {
    GroupGame {
        id: id.to_string(),
        name: id.to_string(),
        parent: Some("knockout".to_string()),
        round: Round::R32,
        lock_mode: LockMode::LockPerMatch,
        carries_standings: false,
        children: GroupChildren::Games(game_ids),
    }
}

fn pred(game_id: &str, home: u8, away: u8) -> MatchPrediction {
    MatchPrediction {
        game_id: game_id.to_string(),
        home_score: home,
        away_score: away,
        locked: true,
    }
}

fn standings_pred(
    group_id: &str,
    ordering: Vec<&str>,
    draw_order: Vec<&str>,
) -> StandingsPrediction {
    StandingsPrediction {
        group_id: group_id.to_string(),
        ordering: ordering.iter().map(|s| s.to_string()).collect(),
        draw_order: draw_order.iter().map(|s| s.to_string()).collect(),
        locked: true,
    }
}

fn result_player(
    match_predictions: Vec<MatchPrediction>,
    standings_predictions: Vec<StandingsPrediction>,
) -> Player {
    Player {
        id: "result".to_string(),
        person_id: "result_person".to_string(),
        nick: "result".to_string(),
        full_name: "Result User".to_string(),
        referrer: None,
        is_result_user: true,
        version: 1,
        match_predictions,
        standings_predictions,
    }
}

/// 12 groups (A–L), 3 teams + 3 games each, plus the one R32 game whose slot is
/// "3ABCDF" (winner E faces a third, per BEST_THIRD_SLOTS) so faces_game has a
/// target. `include_all_groups = false` drops group L to exercise the
/// provisional (incomplete) path.
fn build_test_tournament(include_all_groups: bool) -> Tournament {
    let mut groups: HashMap<GroupId, GroupGame> = HashMap::new();
    let mut games: HashMap<GameId, SingleGame> = HashMap::new();
    let mut teams: HashMap<TeamId, Team> = HashMap::new();

    let last = if include_all_groups { 'L' } else { 'K' };
    let mut match_num = 1u32;
    for letter in 'A'..=last {
        let group_id = format!("group-{}", letter);
        let t1 = format!("{}1", letter);
        let t2 = format!("{}2", letter);
        let t3 = format!("{}3", letter);
        teams.insert(t1.clone(), team(&t1));
        teams.insert(t2.clone(), team(&t2));
        teams.insert(t3.clone(), team(&t3));

        let ids: Vec<String> = (match_num..match_num + 3)
            .map(|n| format!("M{}", n))
            .collect();
        match_num += 3;
        games.insert(
            ids[0].clone(),
            game(&ids[0], &group_id, slot_team(&t1, &t1), slot_team(&t2, &t2)),
        );
        games.insert(
            ids[1].clone(),
            game(&ids[1], &group_id, slot_team(&t1, &t1), slot_team(&t3, &t3)),
        );
        games.insert(
            ids[2].clone(),
            game(&ids[2], &group_id, slot_team(&t2, &t2), slot_team(&t3, &t3)),
        );
        groups.insert(group_id.clone(), group_stage_group(&group_id, ids));
    }

    // The R32 match for the "3ABCDF" slot (winner E vs a best third).
    games.insert(
        "M74".to_string(),
        game(
            "M74",
            "r32-m74",
            slot_placeholder("1E"),
            slot_placeholder("3ABCDF"),
        ),
    );
    groups.insert(
        "r32-m74".to_string(),
        knockout_group("r32-m74", vec!["M74".to_string()]),
    );

    Tournament {
        root: "group-A".to_string(),
        groups,
        games,
        teams,
    }
}

/// Results for one group: X1 wins both (6pts), X2 beats X3 → X2 2nd, X3 3rd.
fn group_predictions(
    letter: char,
    ids: &[String],
) -> (Vec<MatchPrediction>, Vec<StandingsPrediction>) {
    let t1 = format!("{}1", letter);
    let t2 = format!("{}2", letter);
    let t3 = format!("{}3", letter);
    let mp = vec![
        pred(&ids[0], 2, 0), // X1 2-0 X2
        pred(&ids[1], 2, 0), // X1 2-0 X3
        pred(&ids[2], 1, 0), // X2 1-0 X3
    ];
    let sp = vec![standings_pred(
        &format!("group-{}", letter),
        vec![&t1, &t2, &t3],
        vec![],
    )];
    (mp, sp)
}

fn all_predictions(last: char) -> (Vec<MatchPrediction>, Vec<StandingsPrediction>) {
    let mut mp = Vec::new();
    let mut sp = Vec::new();
    let mut m = 1u32;
    for letter in 'A'..=last {
        let ids: Vec<String> = (m..m + 3).map(|n| format!("M{}", n)).collect();
        let (a, b) = group_predictions(letter, &ids);
        mp.extend(a);
        sp.extend(b);
        m += 3;
    }
    (mp, sp)
}

#[test]
fn ranks_all_twelve_thirds_and_flags_top_eight() {
    let t = build_test_tournament(true);
    let (mp, sp) = all_predictions('L');
    let result = result_player(mp, sp);

    let rows = third_place_ranking(&t, &result);

    // All 12 groups determinable → 12 rows, ranked 1..=12.
    assert_eq!(rows.len(), 12, "one row per group");
    assert_eq!(rows[0].rank, 1);
    assert_eq!(rows[11].rank, 12);
    // Every third is X3 (3rd-placed team of its group).
    assert!(rows.iter().all(|r| r.team_id.ends_with('3')));
    // All tie on stats → group-letter order A..L; top 8 (A–H) qualify.
    let qualifying: Vec<char> = rows
        .iter()
        .filter(|r| r.qualifies)
        .map(|r| r.group)
        .collect();
    assert_eq!(qualifying, vec!['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H']);
    assert_eq!(rows.iter().filter(|r| r.qualifies).count(), 8);
}

#[test]
fn attaches_annexe_c_pairing_for_qualifiers() {
    let t = build_test_tournament(true);
    let (mp, sp) = all_predictions('L');
    let result = result_player(mp, sp);

    let rows = third_place_ranking(&t, &result);

    // Annexe C maps winner E to exactly one of the qualifying third-groups.
    // Whichever group it is, that row must point at game M74 (the "3ABCDF" slot).
    let faces_e: Vec<&_> = rows
        .iter()
        .filter(|r| r.faces_winner_group == Some('E'))
        .collect();
    assert_eq!(faces_e.len(), 1, "exactly one third faces winner E");
    assert_eq!(faces_e[0].faces_game.as_deref(), Some("M74"));
    assert!(faces_e[0].qualifies);
    // Non-qualifiers never carry a pairing.
    assert!(rows
        .iter()
        .filter(|r| !r.qualifies)
        .all(|r| r.faces_game.is_none()));
}

#[test]
fn provisional_when_a_group_is_undecided() {
    // Only 11 groups (A–K) have results → < 12 determinable thirds.
    let t = build_test_tournament(false);
    let (mp, sp) = all_predictions('K');
    let result = result_player(mp, sp);

    let rows = third_place_ranking(&t, &result);

    assert_eq!(rows.len(), 11, "only determinable groups produce rows");
    // With 11 thirds, the top-8 set is still resolvable (8 of 11), so Annexe C
    // MAY resolve; but the table is not complete (the resolver's `complete`
    // flag, computed in the GraphQL layer, gates on 12). Here we just assert
    // ranks are dense and qualifies count is 8.
    assert_eq!(rows.iter().filter(|r| r.qualifies).count(), 8);
    assert_eq!(rows[0].rank, 1);
    assert_eq!(rows[10].rank, 11);
}
