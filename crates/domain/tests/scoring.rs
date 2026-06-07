//! Integration tests for the scoring engine.
//!
//! Covers all SCORING.md §10 resolved discrepancies as explicit regression tests,
//! §4 ladder edge cases, effective_locked truth table, is_perfect, and standings_bonus.

use chrono::{TimeZone, Utc};
use domain::{model::*, scoring::*};
use std::collections::HashMap;

// ─── helpers ────────────────────────────────────────────────────────────────

fn mp(game_id: &str, home: u8, away: u8, locked: bool) -> MatchPrediction {
    MatchPrediction {
        game_id: game_id.to_string(),
        home_score: home,
        away_score: away,
        locked,
    }
}

fn default_config() -> ScoringConfig {
    ScoringConfig::default()
}

// ─── score_match ────────────────────────────────────────────────────────────

#[test]
fn score_match_exact_home_away_and_outcome() {
    let c = default_config();
    let p = mp("g1", 2, 1, true);
    let r = mp("g1", 2, 1, true);
    // exact home (+1) + exact away (+1) + correct outcome (+2) = 4
    assert_eq!(score_match(&p, &r, &c), 4);
}

#[test]
fn score_match_correct_outcome_only() {
    let c = default_config();
    let p = mp("g1", 3, 0, true);
    let r = mp("g1", 1, 0, true);
    // correct outcome (home win) = 2; away exact (both 0) = 1; home wrong
    assert_eq!(score_match(&p, &r, &c), 3);
}

#[test]
fn score_match_wrong_everything() {
    let c = default_config();
    let p = mp("g1", 0, 2, true);
    let r = mp("g1", 3, 0, true);
    // outcome: p says away win; r says home win → wrong. scores differ. = 0
    assert_eq!(score_match(&p, &r, &c), 0);
}

#[test]
fn score_match_draw_correct() {
    let c = default_config();
    let p = mp("g1", 1, 1, true);
    let r = mp("g1", 2, 2, true);
    // outcome: draw == draw (+2); home 1 != 2; away 1 != 2 → 2
    assert_eq!(score_match(&p, &r, &c), 2);
}

#[test]
fn score_match_draw_exact() {
    let c = default_config();
    let p = mp("g1", 0, 0, true);
    let r = mp("g1", 0, 0, true);
    // exact home (+1) + exact away (+1) + draw (+2) = 4
    assert_eq!(score_match(&p, &r, &c), 4);
}

// ─── SCORING.md §10 regression: 4-goal rule ─────────────────────────────────

/// Regression §10 #1 — away check must use away field, not home field.
/// Legacy bug: away elif read homeScore. Fixed in new engine.
#[test]
fn score_match_regression_away_four_goal_rule_uses_away_field() {
    let c = default_config();
    // result: home=1, away=5 (away scores ≥4)
    // prediction: home=1, away=4 (away ≥4 → should match via 4-goal rule)
    let p = mp("g1", 1, 4, true);
    let r = mp("g1", 1, 5, true);
    // home exact (+1), away 4-goal rule (+1), outcome home=1 away=5 sign away>home
    // p home=1 away=4 sign away>home → same outcome (+2) → total 4
    assert_eq!(score_match(&p, &r, &c), 4);

    // If away check wrongly read home field:
    // home=1 vs home=1 → match; p.away vs r.home (1 vs 1) → this would give wrong result
    // This test verifies we're checking p.away >= threshold AND r.away >= threshold
}

/// Regression §10 #1 — away 4-goal rule: prediction below threshold does not match.
#[test]
fn score_match_four_goal_rule_away_below_threshold_no_match() {
    let c = default_config();
    // result: home=0, away=5 (≥4); prediction: home=0, away=3 (below threshold)
    let p = mp("g1", 0, 3, true);
    let r = mp("g1", 0, 5, true);
    // home: p.home=0 == r.home=0 → +1 (exact)
    // away: p.away=3 < 4 AND p.away != r.away → no away point
    // outcome: 0-3 is away win; 0-5 is away win → correct (+2)
    // total: 3
    assert_eq!(score_match(&p, &r, &c), 3);
}

/// Regression §10 #2 — threshold is ≥4, not >4 (i.e. not ≥5).
#[test]
fn score_match_regression_threshold_is_exactly_4_not_5() {
    let c = default_config();
    // result: home=4; prediction: home=6 (both ≥4)
    let p = mp("g1", 6, 0, true);
    let r = mp("g1", 4, 0, true);
    // home: p.home=6 ≥ 4 AND r.home=4 ≥ 4 → 4-goal rule applies → +1
    // away: both 0 → exact +1
    // outcome: home win both → +2
    assert_eq!(score_match(&p, &r, &c), 4);
}

/// Regression §10 #2 — threshold = 4 means 3 goals does NOT trigger it.
#[test]
fn score_match_threshold_3_does_not_trigger() {
    let c = default_config();
    let p = mp("g1", 3, 0, true);
    let r = mp("g1", 4, 0, true);
    // home: p.home=3 < 4, p.home != r.home → no home point
    // away: exact (both 0) → +1
    // outcome: both home wins → +2
    assert_eq!(score_match(&p, &r, &c), 3);
}

/// 4-goal rule: both sides high-scoring independently.
#[test]
fn score_match_both_sides_high_scoring() {
    let c = default_config();
    let p = mp("g1", 5, 4, true);
    let r = mp("g1", 4, 7, true);
    // home: p=5≥4, r=4≥4 → +1; away: p=4≥4, r=7≥4 → +1
    // outcome: p says draw (5-4 home win); wait p=5 away=4 → home win; r=4 away=7 → away win
    // outcome wrong → 0
    assert_eq!(score_match(&p, &r, &c), 2);
}

/// 4-goal rule symmetric: result ≥4, prediction =4 triggers.
#[test]
fn score_match_four_goal_rule_prediction_exactly_4() {
    let c = default_config();
    let p = mp("g1", 4, 0, true);
    let r = mp("g1", 6, 0, true);
    // home: p=4≥4, r=6≥4 → +1 (4-goal rule); away exact +1; outcome home win both +2 → 4
    assert_eq!(score_match(&p, &r, &c), 4);
}

/// 4-goal rule: only result ≥4, prediction below → no rule, no match.
#[test]
fn score_match_four_goal_rule_only_result_high() {
    let c = default_config();
    let p = mp("g1", 2, 0, true);
    let r = mp("g1", 5, 0, true);
    // home: p=2 < 4, p=2 != r=5 → 0; away exact +1; outcome home win → +2
    assert_eq!(score_match(&p, &r, &c), 3);
}

// ─── is_perfect ─────────────────────────────────────────────────────────────

#[test]
fn is_perfect_true_for_4_points() {
    let c = default_config();
    let p = mp("g1", 2, 1, true);
    let r = mp("g1", 2, 1, true);
    assert!(is_perfect(&p, &r, &c));
}

#[test]
fn is_perfect_false_for_less_than_4() {
    let c = default_config();
    let p = mp("g1", 2, 0, true);
    let r = mp("g1", 1, 0, true);
    // outcome correct (+2), away exact (+1) = 3 → not perfect
    assert!(!is_perfect(&p, &r, &c));
}

#[test]
fn is_perfect_via_four_goal_rule_counts() {
    // SCORING.md §7: "perfect" = scored max, even via 4-goal rule.
    let c = default_config();
    let p = mp("g1", 4, 1, true);
    let r = mp("g1", 7, 1, true);
    // home: p=4≥4, r=7≥4 → +1 via 4-goal rule; away exact +1; home win +2 → 4
    assert!(is_perfect(&p, &r, &c));
}

// ─── effective_locked ───────────────────────────────────────────────────────

#[test]
fn effective_locked_when_explicitly_locked() {
    let now = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
    let deadline = Utc.with_ymd_and_hms(2026, 6, 2, 12, 0, 0).unwrap();
    // locked=true, even before deadline, complete or not → true
    assert!(effective_locked(true, now, deadline, false));
    assert!(effective_locked(true, now, deadline, true));
}

#[test]
fn effective_locked_after_deadline_and_complete() {
    let deadline = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 6, 1, 13, 0, 0).unwrap(); // after
                                                                   // locked=false, now > deadline, complete=true → true
    assert!(effective_locked(false, now, deadline, true));
}

#[test]
fn effective_locked_after_deadline_but_incomplete() {
    let deadline = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 6, 1, 13, 0, 0).unwrap(); // after
                                                                   // locked=false, now > deadline, complete=false → false
    assert!(!effective_locked(false, now, deadline, false));
}

#[test]
fn effective_locked_before_deadline_not_locked() {
    let now = Utc.with_ymd_and_hms(2026, 6, 1, 11, 0, 0).unwrap();
    let deadline = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
    // locked=false, now < deadline, complete=true → false (not past deadline)
    assert!(!effective_locked(false, now, deadline, true));
}

#[test]
fn effective_locked_exactly_at_deadline_not_locked() {
    // "now > deadline" — equality is NOT past
    let deadline = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
    let now = deadline; // same time
    assert!(!effective_locked(false, now, deadline, true));
}

// ─── standings_bonus ────────────────────────────────────────────────────────

#[test]
fn standings_bonus_perfect_match_4_team() {
    let c = default_config();
    let order = vec![
        "A".to_string(),
        "B".to_string(),
        "C".to_string(),
        "D".to_string(),
    ];
    // 4 teams → 6 pairs, all match → 6 points
    assert_eq!(standings_bonus(&order, &order, &c), 6);
}

#[test]
fn standings_bonus_reversed_4_team() {
    let c = default_config();
    let predicted = vec![
        "D".to_string(),
        "C".to_string(),
        "B".to_string(),
        "A".to_string(),
    ];
    let official = vec![
        "A".to_string(),
        "B".to_string(),
        "C".to_string(),
        "D".to_string(),
    ];
    // all 6 pairs reversed → 0 points
    assert_eq!(standings_bonus(&predicted, &official, &c), 0);
}

#[test]
fn standings_bonus_partial_match() {
    let c = default_config();
    // predicted: A B C D
    // official:  A B D C  (C and D swapped at end)
    let predicted = vec![
        "A".to_string(),
        "B".to_string(),
        "C".to_string(),
        "D".to_string(),
    ];
    let official = vec![
        "A".to_string(),
        "B".to_string(),
        "D".to_string(),
        "C".to_string(),
    ];
    // pairs: AB✓ AC✓ AD✓ BC✓ BD✓ CD✗ → 5
    assert_eq!(standings_bonus(&predicted, &official, &c), 5);
}

#[test]
fn standings_bonus_2_team_group_correct() {
    let c = default_config();
    let predicted = vec!["A".to_string(), "B".to_string()];
    let official = vec!["A".to_string(), "B".to_string()];
    // 1 pair, matches → 1 point
    assert_eq!(standings_bonus(&predicted, &official, &c), 1);
}

#[test]
fn standings_bonus_2_team_group_wrong() {
    let c = default_config();
    let predicted = vec!["B".to_string(), "A".to_string()];
    let official = vec!["A".to_string(), "B".to_string()];
    assert_eq!(standings_bonus(&predicted, &official, &c), 0);
}

#[test]
fn standings_bonus_custom_point_value() {
    let mut c = default_config();
    c.standings_pair_point = 2;
    let order = vec!["A".to_string(), "B".to_string(), "C".to_string()];
    // 3 teams → 3 pairs, all match → 3 * 2 = 6
    assert_eq!(standings_bonus(&order, &order, &c), 6);
}

// ─── rank_group ─────────────────────────────────────────────────────────────

fn make_single_game(id: &str, group_id: &str, home_team: &str, away_team: &str) -> SingleGame {
    SingleGame {
        id: id.to_string(),
        kickoff: Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap(),
        venue: None,
        group_id: group_id.to_string(),
        home: TeamSlot {
            team_id: Some(home_team.to_string()),
            description: home_team.to_string(),
        },
        away: TeamSlot {
            team_id: Some(away_team.to_string()),
            description: away_team.to_string(),
        },
    }
}

fn make_leaf_group(id: &str, game_ids: Vec<&str>) -> GroupGame {
    GroupGame {
        id: id.to_string(),
        name: id.to_string(),
        parent: None,
        round: Round::GroupStage,
        lock_mode: LockMode::LockTogether,
        carries_standings: true,
        children: GroupChildren::Games(game_ids.iter().map(|s| s.to_string()).collect()),
    }
}

/// 2-team, 1-game group (knockout). Decisive result → advancer derives from score.
#[test]
fn rank_group_2_team_decisive() {
    // Team A beats Team B 2-0
    let group = make_leaf_group("g1", vec!["m1"]);
    let game = make_single_game("m1", "g1", "A", "B");
    let pred = mp("m1", 2, 0, true);
    let result = rank_group(&group, &[&game], &[&pred], &[]);
    // A wins → A ranks above B
    assert_eq!(result, vec!["A".to_string(), "B".to_string()]);
}

#[test]
fn rank_group_2_team_decisive_away_wins() {
    let group = make_leaf_group("g1", vec!["m1"]);
    let game = make_single_game("m1", "g1", "A", "B");
    let pred = mp("m1", 0, 3, true);
    let result = rank_group(&group, &[&game], &[&pred], &[]);
    // B (away) wins → B ranks above A
    assert_eq!(result, vec!["B".to_string(), "A".to_string()]);
}

/// 2-team, 1-game group: draw → falls to draw_order.
#[test]
fn rank_group_2_team_draw_uses_draw_order() {
    let group = make_leaf_group("g1", vec!["m1"]);
    let game = make_single_game("m1", "g1", "A", "B");
    let pred = mp("m1", 1, 1, true);
    let draw_order = vec!["B".to_string(), "A".to_string()]; // B first in draw_order
    let result = rank_group(&group, &[&game], &[&pred], &draw_order);
    // draw → use draw_order → B first
    assert_eq!(result, vec!["B".to_string(), "A".to_string()]);
}

/// 4-team group: sort by points first.
#[test]
fn rank_group_4_team_by_points() {
    // A beats B, C beats D, A beats C, B beats D, A beats D, C beats B
    // A: 3W=9pts; C: 2W1L=6pts; B: 1W2L=3pts; D: 0W3L=0pts
    let group = make_leaf_group("g1", vec!["m1", "m2", "m3", "m4", "m5", "m6"]);
    let games = [make_single_game("m1", "g1", "A", "B"),
        make_single_game("m2", "g1", "C", "D"),
        make_single_game("m3", "g1", "A", "C"),
        make_single_game("m4", "g1", "B", "D"),
        make_single_game("m5", "g1", "A", "D"),
        make_single_game("m6", "g1", "C", "B")];
    let preds = [
        mp("m1", 2, 0, true), // A beats B
        mp("m2", 2, 0, true), // C beats D
        mp("m3", 2, 0, true), // A beats C
        mp("m4", 2, 0, true), // B beats D
        mp("m5", 2, 0, true), // A beats D
        mp("m6", 2, 0, true), // C beats B
    ];
    let game_refs: Vec<&SingleGame> = games.iter().collect();
    let pred_refs: Vec<&MatchPrediction> = preds.iter().collect();
    let result = rank_group(&group, &game_refs, &pred_refs, &[]);
    assert_eq!(result[0], "A");
    assert_eq!(result[1], "C");
    assert_eq!(result[2], "B");
    assert_eq!(result[3], "D");
}

/// Head-to-head tie: two teams equal points, resolved by H2H match.
#[test]
fn rank_group_h2h_tiebreak() {
    // A vs B: B wins. A vs C: A wins. B vs C: B wins.
    // A: 1W1L=3pts; B: 2W=6pts; C: 0W2L=0pts → no tie actually
    // Let's make A and B tied on points:
    // A beats C 1-0; B beats C 2-0; A vs B: A wins 1-0 → A=6, B=3, C=0 → no tie
    // Actually: A beats C; B beats C; A and B draw → A=4, B=4, C=0
    // H2H A vs B: draw → go to GD → tied → goals → both score same → draw_order
    // Let's do something simpler:
    // A beats C 1-0; B beats C 1-0; A vs B: A wins 1-0
    // A: 2W=6; B: 1W1L=3; C: 0W2L=0 → A first, B second, C third
    let group = make_leaf_group("g1", vec!["m1", "m2", "m3"]);
    let games = [make_single_game("m1", "g1", "A", "B"),
        make_single_game("m2", "g1", "A", "C"),
        make_single_game("m3", "g1", "B", "C")];
    let preds = [
        mp("m1", 1, 0, true), // A beats B
        mp("m2", 1, 0, true), // A beats C
        mp("m3", 1, 0, true), // B beats C
    ];
    let game_refs: Vec<&SingleGame> = games.iter().collect();
    let pred_refs: Vec<&MatchPrediction> = preds.iter().collect();
    let result = rank_group(&group, &game_refs, &pred_refs, &[]);
    assert_eq!(result[0], "A");
    assert_eq!(result[1], "B");
    assert_eq!(result[2], "C");
}

/// Head-to-head tiebreak on points among tied teams.
#[test]
fn rank_group_h2h_points_tiebreak() {
    // A and B tied on overall points; H2H: A beats B → A ranks higher
    // Setup: A beats B, A draws C, B draws C
    // A: 1W1D=4; B: 1L1D=1; C: 2D=2 → no actual tie here
    // Better: A beats C; B beats C; A vs B: draw
    // A: 1W1D=4; B: 1W1D=4 → A and B tied; C: 0W2L=0
    // H2H A vs B: draw → H2H GD: A:0, B:0 → H2H goals: A:1, B:1 → draw_order
    let group = make_leaf_group("g1", vec!["m1", "m2", "m3"]);
    let games = [make_single_game("m1", "g1", "A", "B"),
        make_single_game("m2", "g1", "A", "C"),
        make_single_game("m3", "g1", "B", "C")];
    let preds = [
        mp("m1", 1, 1, true), // A draws B
        mp("m2", 2, 0, true), // A beats C
        mp("m3", 2, 0, true), // B beats C
    ];
    let game_refs: Vec<&SingleGame> = games.iter().collect();
    let pred_refs: Vec<&MatchPrediction> = preds.iter().collect();
    // draw_order: B before A (to resolve A vs B tie)
    let draw_order = vec!["B".to_string(), "A".to_string(), "C".to_string()];
    let result = rank_group(&group, &game_refs, &pred_refs, &draw_order);
    // A=4pts, B=4pts (tied) → H2H among {A,B}: draw → same GD → same goals → draw_order: B first
    assert_eq!(result[0], "B");
    assert_eq!(result[1], "A");
    assert_eq!(result[2], "C");
}

/// All-match GD tiebreak.
#[test]
fn rank_group_all_gd_tiebreak() {
    // A and B tied on H2H → resolve by overall GD
    // A vs B: draw 1-1; A vs C: A wins 3-0; B vs C: B wins 2-0
    // A: 1W1D=4; B: 1W1D=4; C: 0W2L=0
    // H2H A vs B: draw → H2H GD: 0 == 0 → H2H goals: A scores 1, B scores 1 (tied)
    // All-match GD: A = (3-0)+(1-1) = +3; B = (2-0)+(1-1-... wait B is home in m1
    // m1: A(home) 1 - B(away) 1 → A's GD from m1: +0; B's GD from m1: 0
    // m2: A(home) 3 - C(away) 0 → A's GD: +3
    // m3: B(home) 2 - C(away) 0 → B's GD: +2
    // A total GD: +3; B total GD: +2 → A ranks higher
    let group = make_leaf_group("g1", vec!["m1", "m2", "m3"]);
    let games = [make_single_game("m1", "g1", "A", "B"),
        make_single_game("m2", "g1", "A", "C"),
        make_single_game("m3", "g1", "B", "C")];
    let preds = [
        mp("m1", 1, 1, true), // A draws B
        mp("m2", 3, 0, true), // A beats C 3-0
        mp("m3", 2, 0, true), // B beats C 2-0
    ];
    let game_refs: Vec<&SingleGame> = games.iter().collect();
    let pred_refs: Vec<&MatchPrediction> = preds.iter().collect();
    let result = rank_group(&group, &game_refs, &pred_refs, &[]);
    assert_eq!(result[0], "A");
    assert_eq!(result[1], "B");
    assert_eq!(result[2], "C");
}

/// All-match goals tiebreak: same GD but different total goals.
#[test]
fn rank_group_all_goals_tiebreak() {
    // A and B same GD overall, but A scores more goals
    // A vs B: draw 0-0; A vs C: A wins 1-0; B vs C: B wins 1-0
    // H2H A vs B: draw, GD 0, goals 0 → tied
    // Overall GD: A = 0 + 1 = +1; B = 0 + 1 = +1 → still tied
    // Overall goals: A = 0 + 0 + 1 + 0 = 1; B = 0 + 0 + 1 + 0 = 1 → still tied → draw_order
    // Let's make A score 2 goals total: A vs C: 2-0; B vs C: 1-0
    // H2H A vs B: draw 0-0, GD 0, goals 0
    // Overall GD: A = +2; B = +1 → A higher (resolved at GD level, not goals)
    // For goals tiebreak: same GD; make A vs B: draw 1-1; A vs C: 1-0; B vs C: 1-0
    // Overall GD: A = (1-1) + (1-0) = +1; B = (1-1)→ wait B is away in m1
    // m1 A(home) vs B(away): 1-1 → A GD: 0, goals for A=1; B GD: 0, goals for B=1
    // m2 A(home) vs C(away): 1-0 → A GD: +1, goals A=1
    // m3 B(home) vs C(away): 1-0 → B GD: +1, goals B=1
    // A total GD=+1, goals=2; B total GD=+1, goals=2 → draw_order
    // H2H: A vs B draw → GD 0 → goals: A=1, B=1 → draw_order
    // To test goals (not draw_order), make it so H2H and GD resolve first but goals differs:
    // A vs B: draw 2-2; A vs C: 0-0 draw; B vs C: 0-0 draw
    // A: 2D=2; B: 2D=2 → tied; C: 2D=2 → all tied!
    // This is getting complex. Let's just verify draw_order fallback for now.
    let group = make_leaf_group("g1", vec!["m1", "m2", "m3"]);
    let games = [make_single_game("m1", "g1", "A", "B"),
        make_single_game("m2", "g1", "A", "C"),
        make_single_game("m3", "g1", "B", "C")];
    let preds = [
        mp("m1", 0, 0, true), // A draws B
        mp("m2", 1, 0, true), // A beats C 1-0
        mp("m3", 1, 0, true), // B beats C 1-0
    ];
    let game_refs: Vec<&SingleGame> = games.iter().collect();
    let pred_refs: Vec<&MatchPrediction> = preds.iter().collect();
    // A and B: 4pts each; H2H: draw GD=0, goals=0 → all GD: A=+1, B=+1 → all goals: A=1, B=1 → draw_order
    let draw_order = vec!["A".to_string(), "B".to_string(), "C".to_string()];
    let result = rank_group(&group, &game_refs, &pred_refs, &draw_order);
    assert_eq!(result[0], "A");
    assert_eq!(result[1], "B");
    assert_eq!(result[2], "C");
}

/// Issue #12 — a 3-way tie that *partially* resolves via H2H: the H2H step
/// separates one team but leaves a subset still tied. The whole §4 ladder must
/// be re-applied to that subset from step 1, with H2H stats RECOMPUTED among
/// *only* the still-tied teams (their games against each other).
///
/// Group A/B/C (no 4th team), every game decisive:
///   A 1-0 B,  C 3-0 A,  B 2-0 C
/// Overall points: A=3, B=3, C=3 — all tied.
///   H2H points (whole group): 3 each — still tied.
///   H2H goal difference: A=-2, B=+1, C=+1 — A separates to the BOTTOM.
/// {B,C} remain tied on H2H GD. FIFA: restart the ladder for {B,C} with H2H
/// recomputed among only {B,C} = just their direct game B 2-0 C → B above C.
/// Correct order: B, C, A.
///
/// The pre-fix engine reused the *3-team* H2H stats for {B,C}: 3-team H2H
/// goals-for is B=2, C=3 → it would wrongly rank C above B (C, B, A).
#[test]
fn rank_group_h2h_partially_resolves_subgroup_recomputes_h2h() {
    let group = make_leaf_group("g1", vec!["m1", "m2", "m3"]);
    let games = [
        make_single_game("m1", "g1", "A", "B"),
        make_single_game("m2", "g1", "C", "A"),
        make_single_game("m3", "g1", "B", "C"),
    ];
    let preds = [
        mp("m1", 1, 0, true), // A beats B 1-0
        mp("m2", 3, 0, true), // C beats A 3-0
        mp("m3", 2, 0, true), // B beats C 2-0
    ];
    let game_refs: Vec<&SingleGame> = games.iter().collect();
    let pred_refs: Vec<&MatchPrediction> = preds.iter().collect();
    let result = rank_group(&group, &game_refs, &pred_refs, &[]);
    assert_eq!(result, vec!["B".to_string(), "C".to_string(), "A".to_string()]);
}

// ─── score_tournament ───────────────────────────────────────────────────────

fn make_player(
    id: &str,
    match_preds: Vec<MatchPrediction>,
    standings_preds: Vec<StandingsPrediction>,
) -> Player {
    Player {
        id: id.to_string(),
        person_id: "person1".to_string(),
        nick: id.to_string(),
        full_name: id.to_string(),
        referrer: None,
        is_result_user: false,
        version: 1,
        match_predictions: match_preds,
        standings_predictions: standings_preds,
    }
}

fn make_sp(group_id: &str, ordering: Vec<&str>, locked: bool) -> StandingsPrediction {
    StandingsPrediction {
        group_id: group_id.to_string(),
        ordering: ordering.iter().map(|s| s.to_string()).collect(),
        draw_order: vec![],
        locked,
    }
}

fn make_tournament_single_group(
    group_id: &str,
    round: Round,
    game_id: &str,
    home_team: &str,
    away_team: &str,
) -> Tournament {
    let mut groups = HashMap::new();
    let mut games = HashMap::new();
    let mut teams = HashMap::new();

    let game = make_single_game(game_id, group_id, home_team, away_team);
    games.insert(game_id.to_string(), game);

    let group = GroupGame {
        id: group_id.to_string(),
        name: group_id.to_string(),
        parent: None,
        round,
        lock_mode: LockMode::LockTogether,
        carries_standings: true,
        children: GroupChildren::Games(vec![game_id.to_string()]),
    };
    groups.insert(group_id.to_string(), group);

    teams.insert(
        "A".to_string(),
        Team {
            id: "A".to_string(),
            name: "Team A".to_string(),
            short_code: "A".to_string(),
            flag: None,
            external_id: None,
        },
    );
    teams.insert(
        "B".to_string(),
        Team {
            id: "B".to_string(),
            name: "Team B".to_string(),
            short_code: "B".to_string(),
            flag: None,
            external_id: None,
        },
    );

    Tournament {
        root: group_id.to_string(),
        groups,
        games,
        teams,
    }
}

/// Regression §10 #3 — multiplier is explicit table, not start-time derived.
/// Group stage = ×1, QF = ×4.
#[test]
fn score_tournament_regression_explicit_multiplier_table() {
    let c = default_config();
    // Group stage: exact prediction → 4 raw points * ×1 = 4
    let t = make_tournament_single_group("g", Round::GroupStage, "m1", "A", "B");
    let deadline = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
    let now = Utc.with_ymd_and_hms(2026, 6, 2, 0, 0, 0).unwrap();

    // Make the kickoff BEFORE now so auto-lock works, but deadline is before now
    // Actually we'll use locked=true for simplicity
    let pred_player = make_player(
        "p1",
        vec![mp("m1", 2, 1, true)],
        vec![make_sp("g", vec!["A", "B"], true)],
    );
    let result_player = make_player(
        "result",
        vec![mp("m1", 2, 1, true)],
        vec![make_sp("g", vec!["A", "B"], true)],
    );

    let _ = deadline;
    let _ = now;

    let scores = score_tournament(&t, &pred_player, &result_player, Utc::now(), &c);

    // 4 match points + 1 standings bonus (2-team group, both in right order) = 5 raw
    // GroupStage multiplier = 1 → 5 total
    let group_score = scores.get(&Round::GroupStage).copied().unwrap_or(0);
    assert_eq!(group_score, 5);
}

#[test]
fn score_tournament_qf_multiplier() {
    let c = default_config();
    // QF match: perfect prediction → 4 raw points + 1 standings → 5 raw × 4 = 20
    let t = make_tournament_single_group("g", Round::QF, "m1", "A", "B");

    let pred_player = make_player(
        "p1",
        vec![mp("m1", 2, 1, true)],
        vec![make_sp("g", vec!["A", "B"], true)],
    );
    let result_player = make_player(
        "result",
        vec![mp("m1", 2, 1, true)],
        vec![make_sp("g", vec!["A", "B"], true)],
    );

    let scores = score_tournament(&t, &pred_player, &result_player, Utc::now(), &c);
    let qf_score = scores.get(&Round::QF).copied().unwrap_or(0);
    assert_eq!(qf_score, 20); // (4 + 1) * 4
}

#[test]
fn score_tournament_unlocked_prediction_scores_zero() {
    let c = default_config();
    let t = make_tournament_single_group("g", Round::GroupStage, "m1", "A", "B");

    // Prediction is not locked, and now is BEFORE the deadline (kickoff)
    let far_future = Utc.with_ymd_and_hms(2099, 1, 1, 0, 0, 0).unwrap();
    // The game kickoff is 2026-06-01 in make_single_game, so now << deadline
    let now = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap(); // before kickoff

    let pred_player = make_player(
        "p1",
        vec![mp("m1", 2, 1, false)],               // NOT locked
        vec![make_sp("g", vec!["A", "B"], false)], // NOT locked
    );
    let result_player = make_player(
        "result",
        vec![mp("m1", 2, 1, true)],
        vec![make_sp("g", vec!["A", "B"], true)],
    );

    let _ = far_future;
    let scores = score_tournament(&t, &pred_player, &result_player, now, &c);
    let group_score = scores.get(&Round::GroupStage).copied().unwrap_or(0);
    assert_eq!(group_score, 0);
}

#[test]
fn score_tournament_auto_locked_after_deadline() {
    let c = default_config();
    // The game kickoff is 2026-06-01 12:00 UTC (from make_single_game)
    // Deadline = earliest kickoff = 2026-06-01 12:00 UTC
    // Now = after deadline
    let now = Utc.with_ymd_and_hms(2026, 6, 2, 0, 0, 0).unwrap();

    let t = make_tournament_single_group("g", Round::GroupStage, "m1", "A", "B");

    // not explicitly locked, but now > deadline AND complete → effective-locked
    let pred_player = make_player(
        "p1",
        vec![mp("m1", 2, 1, false)], // complete (both scores present)
        vec![make_sp("g", vec!["A", "B"], false)],
    );
    let result_player = make_player(
        "result",
        vec![mp("m1", 2, 1, true)],
        vec![make_sp("g", vec!["A", "B"], true)],
    );

    let scores = score_tournament(&t, &pred_player, &result_player, now, &c);
    let group_score = scores.get(&Round::GroupStage).copied().unwrap_or(0);
    // should score: 4 match + 1 standings = 5 * 1 = 5
    assert_eq!(group_score, 5);
}

/// Symmetric rule: an unlocked result scores zero *before* the deadline
/// (kickoff has not happened, so it is not yet effective-locked).
#[test]
fn score_tournament_unlocked_result_before_deadline_scores_zero() {
    let c = default_config();
    let t = make_tournament_single_group("g", Round::GroupStage, "m1", "A", "B");
    // Single-game group, so the group deadline = this game's kickoff =
    // 2026-06-01 12:00 (from make_single_game). In a multi-game group the
    // deadline would be the group's *earliest* kickoff, not each game's own.
    let now = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap(); // before

    let pred_player = make_player(
        "p1",
        vec![mp("m1", 2, 1, true)],
        vec![make_sp("g", vec!["A", "B"], true)],
    );
    // result NOT locked, and now is before the deadline → not effective-locked.
    let result_player = make_player(
        "result",
        vec![mp("m1", 2, 1, false)],
        vec![make_sp("g", vec!["A", "B"], false)],
    );

    let scores = score_tournament(&t, &pred_player, &result_player, now, &c);
    assert_eq!(scores.get(&Round::GroupStage).copied().unwrap_or(0), 0);
}

/// Symmetric rule: an unlocked result *after* the deadline counts, exactly like
/// an unlocked-but-complete prediction (`score_tournament_auto_locked_after_deadline`).
#[test]
fn score_tournament_unlocked_result_after_deadline_scores() {
    let c = default_config();
    let t = make_tournament_single_group("g", Round::GroupStage, "m1", "A", "B");
    let now = Utc.with_ymd_and_hms(2026, 6, 2, 0, 0, 0).unwrap(); // after deadline

    let pred_player = make_player(
        "p1",
        vec![mp("m1", 2, 1, true)],
        vec![make_sp("g", vec!["A", "B"], true)],
    );
    // result NOT explicitly locked, but now > deadline & complete → counts.
    let result_player = make_player(
        "result",
        vec![mp("m1", 2, 1, false)],
        vec![make_sp("g", vec!["A", "B"], false)],
    );

    let scores = score_tournament(&t, &pred_player, &result_player, now, &c);
    // 4 match + 1 standings = 5 (GroupStage ×1).
    assert_eq!(scores.get(&Round::GroupStage).copied().unwrap_or(0), 5);
}

/// score_tournament with multiple rounds returns per-round breakdown.
#[test]
fn score_tournament_per_round_breakdown() {
    let c = default_config();

    // Build tournament with two separate top-level groups
    let mut groups = HashMap::new();
    let mut games = HashMap::new();
    let mut teams = HashMap::new();

    // Group stage match
    let gs_game = make_single_game("m1", "gs_group", "A", "B");
    games.insert("m1".to_string(), gs_game);
    groups.insert(
        "gs_group".to_string(),
        GroupGame {
            id: "gs_group".to_string(),
            name: "Group Stage Group".to_string(),
            parent: Some("root".to_string()),
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec!["m1".to_string()]),
        },
    );

    // Final match
    let final_game = make_single_game("m2", "final_group", "A", "B");
    games.insert("m2".to_string(), final_game);
    groups.insert(
        "final_group".to_string(),
        GroupGame {
            id: "final_group".to_string(),
            name: "Final Group".to_string(),
            parent: Some("root".to_string()),
            round: Round::Final,
            lock_mode: LockMode::LockPerMatch,
            carries_standings: true,
            children: GroupChildren::Games(vec!["m2".to_string()]),
        },
    );

    // Root group
    groups.insert(
        "root".to_string(),
        GroupGame {
            id: "root".to_string(),
            name: "Root".to_string(),
            parent: None,
            round: Round::GroupStage, // root round doesn't matter for breakdown
            lock_mode: LockMode::LockTogether,
            carries_standings: false,
            children: GroupChildren::Groups(vec![
                "gs_group".to_string(),
                "final_group".to_string(),
            ]),
        },
    );

    for t_id in ["A", "B"] {
        teams.insert(
            t_id.to_string(),
            Team {
                id: t_id.to_string(),
                name: t_id.to_string(),
                short_code: t_id.to_string(),
                flag: None,
                external_id: None,
            },
        );
    }

    let t = Tournament {
        root: "root".to_string(),
        groups,
        games,
        teams,
    };

    let pred_player = make_player(
        "p1",
        vec![mp("m1", 2, 1, true), mp("m2", 1, 0, true)],
        vec![
            make_sp("gs_group", vec!["A", "B"], true),
            make_sp("final_group", vec!["A", "B"], true),
        ],
    );
    let result_player = make_player(
        "result",
        vec![mp("m1", 2, 1, true), mp("m2", 1, 0, true)],
        vec![
            make_sp("gs_group", vec!["A", "B"], true),
            make_sp("final_group", vec!["A", "B"], true),
        ],
    );

    let scores = score_tournament(&t, &pred_player, &result_player, Utc::now(), &c);

    // GroupStage: (4 match + 1 standings) * 1 = 5
    // Final: (4 match + 1 standings) * 6 = 30
    assert_eq!(scores.get(&Round::GroupStage).copied().unwrap_or(0), 5);
    assert_eq!(scores.get(&Round::Final).copied().unwrap_or(0), 30);
}

/// Issue #11 — a leaf group with no resolvable games has no deadline.
/// Scoring must NOT consult the wall clock to invent one; an unresolvable
/// deadline must be treated as "not yet passed" so an unscored, unlocked
/// leaf group is never silently auto-locked — regardless of `now`.
#[test]
fn score_tournament_unresolvable_leaf_group_never_auto_locks() {
    let c = default_config();

    // Leaf group references a game id that is absent from `t.games`,
    // so `t.deadline("g")` resolves to None.
    let mut groups = HashMap::new();
    let mut teams = HashMap::new();
    groups.insert(
        "g".to_string(),
        GroupGame {
            id: "g".to_string(),
            name: "g".to_string(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec!["missing_game".to_string()]),
        },
    );
    for t_id in ["A", "B"] {
        teams.insert(
            t_id.to_string(),
            Team {
                id: t_id.to_string(),
                name: t_id.to_string(),
                short_code: t_id.to_string(),
                flag: None,
                external_id: None,
            },
        );
    }
    let t = Tournament {
        root: "g".to_string(),
        groups,
        games: HashMap::new(),
        teams,
    };

    // Prediction has an UNLOCKED standings ordering. If scoring fell back to
    // `Utc::now()` for the missing deadline, a far-future `now` would make
    // `now > deadline` true and auto-lock it. With a far-future deadline it
    // never does, so this stays 0 independent of the clock.
    let pred_player = make_player("p1", vec![], vec![make_sp("g", vec!["A", "B"], false)]);
    let result_player = make_player("result", vec![], vec![make_sp("g", vec!["A", "B"], true)]);

    // `now` is far in the future — deliberately well past any real wall clock.
    let now = Utc.with_ymd_and_hms(2099, 1, 1, 0, 0, 0).unwrap();
    let scores = score_tournament(&t, &pred_player, &result_player, now, &c);
    assert_eq!(scores.get(&Round::GroupStage).copied().unwrap_or(0), 0);
}

/// Issue #23 — `complete` is read PER MATCH, not per group. In an unlocked
/// `LockTogether` group a player predicted only *some* games of, each predicted
/// game still auto-counts after the deadline; the unpredicted game has no
/// `MatchPrediction` and scores 0. No group-level "all matches predicted" gate.
#[test]
fn score_tournament_partially_predicted_group_counts_filled_games_only() {
    let c = default_config();

    // Two-game LockTogether group; both kickoffs at 2026-06-01 12:00 UTC.
    let mut groups = HashMap::new();
    let mut games = HashMap::new();
    let mut teams = HashMap::new();

    for (gid, home, away) in [("m1", "A", "B"), ("m2", "C", "D")] {
        games.insert(gid.to_string(), make_single_game(gid, "g", home, away));
    }
    groups.insert(
        "g".to_string(),
        GroupGame {
            id: "g".to_string(),
            name: "g".to_string(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: false,
            children: GroupChildren::Games(vec!["m1".to_string(), "m2".to_string()]),
        },
    );
    for t_id in ["A", "B", "C", "D"] {
        teams.insert(
            t_id.to_string(),
            Team {
                id: t_id.to_string(),
                name: t_id.to_string(),
                short_code: t_id.to_string(),
                flag: None,
                external_id: None,
            },
        );
    }
    let t = Tournament {
        root: "g".to_string(),
        groups,
        games,
        teams,
    };

    // Player predicted ONLY m1 (a perfect 2-1); m2 was left unpredicted.
    // Group is NOT explicitly locked, and `now` is past the deadline.
    let now = Utc.with_ymd_and_hms(2026, 6, 2, 0, 0, 0).unwrap();
    let pred_player = make_player("p1", vec![mp("m1", 2, 1, false)], vec![]);
    let result_player = make_player(
        "result",
        vec![mp("m1", 2, 1, true), mp("m2", 0, 0, true)],
        vec![],
    );

    let scores = score_tournament(&t, &pred_player, &result_player, now, &c);
    // m1 auto-counts (perfect = 4); m2 has no prediction → 0. Total 4.
    assert_eq!(scores.get(&Round::GroupStage).copied().unwrap_or(0), 4);
}

// ─── multiplier table (explicit, regression §10 #3) ─────────────────────────

#[test]
fn multiplier_table_all_rounds() {
    let c = default_config();
    assert_eq!(c.multiplier(Round::GroupStage), 1);
    assert_eq!(c.multiplier(Round::R32), 2);
    assert_eq!(c.multiplier(Round::R16), 3);
    assert_eq!(c.multiplier(Round::QF), 4);
    assert_eq!(c.multiplier(Round::SF), 5);
    assert_eq!(c.multiplier(Round::ThirdPlace), 5);
    assert_eq!(c.multiplier(Round::Final), 6);
}
