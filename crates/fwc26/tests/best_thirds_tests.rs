//! Tests for best_thirds (FWC26_RULES.md §3).

use fwc26::{best_thirds, TeamStats};

fn stats(
    team_id: &str,
    points: i32,
    goal_diff: i32,
    goals_for: i32,
    conduct: i32,
) -> (char, TeamStats) {
    (
        team_id.chars().next().unwrap(),
        TeamStats {
            team_id: team_id.to_string(),
            points,
            goal_diff,
            goals_for,
            conduct,
        },
    )
}

/// Returns exactly 8 group letters.
#[test]
fn test_best_thirds_returns_8() {
    let thirds: Vec<(char, TeamStats)> = "ABCDEFGHIJKL"
        .chars()
        .map(|c| {
            (
                c,
                TeamStats {
                    team_id: c.to_string(),
                    points: 3,
                    goal_diff: 0,
                    goals_for: 1,
                    conduct: 0,
                },
            )
        })
        .collect();
    let result = best_thirds(&thirds);
    assert_eq!(result.len(), 8);
}

/// Ranks by points first.
#[test]
fn test_best_thirds_by_points() {
    let thirds = vec![
        stats("A", 6, 2, 4, 0),
        stats("B", 4, 1, 3, 0),
        stats("C", 3, 0, 2, 0),
        stats("D", 3, 0, 2, 0),
        stats("E", 3, 0, 2, 0),
        stats("F", 3, 0, 2, 0),
        stats("G", 1, -1, 1, 0),
        stats("H", 1, -1, 1, 0),
        stats("I", 1, -2, 1, 0),
        stats("J", 0, -3, 0, 0),
        stats("K", 0, -3, 0, 0),
        stats("L", 0, -4, 0, 0),
    ];
    let result = best_thirds(&thirds);
    assert_eq!(result[0], 'A', "Highest points should be first");
    assert_eq!(result[1], 'B', "Second highest points should be second");
    // Top 8 should all have points >= 1
    assert!(!result.contains(&'J'), "J (0 pts) must not be in top 8");
    assert!(!result.contains(&'K'), "K (0 pts) must not be in top 8");
    assert!(!result.contains(&'L'), "L (0 pts) must not be in top 8");
}

/// Ties in points are broken by goal difference.
#[test]
fn test_best_thirds_gd_tiebreak() {
    let thirds = vec![
        stats("A", 9, 5, 6, 0),
        stats("B", 6, 3, 5, 0),
        stats("C", 6, 1, 3, 0), // same points as B, lower GD
        stats("D", 3, 0, 2, 0),
        stats("E", 3, 0, 2, 0),
        stats("F", 3, 0, 2, 0),
        stats("G", 3, 0, 2, 0),
        stats("H", 3, 0, 2, 0),
        stats("I", 0, -1, 1, 0),
        stats("J", 0, -2, 1, 0),
        stats("K", 0, -3, 0, 0),
        stats("L", 0, -4, 0, 0),
    ];
    let result = best_thirds(&thirds);
    // B should rank ahead of C (same points, B has better GD)
    let b_pos = result.iter().position(|&c| c == 'B').unwrap();
    let c_pos = result.iter().position(|&c| c == 'C').unwrap();
    assert!(b_pos < c_pos, "B (GD=3) should rank ahead of C (GD=1)");
}

/// Ties in GD broken by goals scored.
#[test]
fn test_best_thirds_goals_for_tiebreak() {
    let thirds = vec![
        stats("A", 6, 2, 5, 0), // higher goals
        stats("B", 6, 2, 3, 0), // same pts and GD, fewer goals
        stats("C", 4, 1, 3, 0),
        stats("D", 3, 0, 2, 0),
        stats("E", 3, 0, 2, 0),
        stats("F", 3, 0, 2, 0),
        stats("G", 3, 0, 2, 0),
        stats("H", 3, 0, 2, 0),
        stats("I", 0, -1, 1, 0),
        stats("J", 0, -2, 1, 0),
        stats("K", 0, -3, 0, 0),
        stats("L", 0, -4, 0, 0),
    ];
    let result = best_thirds(&thirds);
    let a_pos = result.iter().position(|&c| c == 'A').unwrap();
    let b_pos = result.iter().position(|&c| c == 'B').unwrap();
    assert!(
        a_pos < b_pos,
        "A (5 goals) should rank ahead of B (3 goals)"
    );
}

/// Ties broken by conduct score (higher/less negative = better).
#[test]
fn test_best_thirds_conduct_tiebreak() {
    let thirds = vec![
        stats("A", 6, 2, 4, 0),  // best conduct
        stats("B", 6, 2, 4, -1), // slightly worse conduct
        stats("C", 4, 1, 3, 0),
        stats("D", 3, 0, 2, 0),
        stats("E", 3, 0, 2, 0),
        stats("F", 3, 0, 2, 0),
        stats("G", 3, 0, 2, 0),
        stats("H", 3, 0, 2, 0),
        stats("I", 0, -1, 1, 0),
        stats("J", 0, -2, 1, 0),
        stats("K", 0, -3, 0, 0),
        stats("L", 0, -4, 0, 0),
    ];
    let result = best_thirds(&thirds);
    let a_pos = result.iter().position(|&c| c == 'A').unwrap();
    let b_pos = result.iter().position(|&c| c == 'B').unwrap();
    assert!(
        a_pos < b_pos,
        "A (conduct=0) should rank ahead of B (conduct=-1)"
    );
}

/// All equal stats → input order preserved (stable sort = stand-in for FIFA ranking).
#[test]
fn test_best_thirds_input_order_for_equal_stats() {
    let thirds: Vec<(char, TeamStats)> = "ABCDEFGHIJKL"
        .chars()
        .map(|c| {
            (
                c,
                TeamStats {
                    team_id: c.to_string(),
                    points: 3,
                    goal_diff: 0,
                    goals_for: 1,
                    conduct: 0,
                },
            )
        })
        .collect();
    let result = best_thirds(&thirds);
    // With all equal stats, input order should be preserved: A,B,C,D,E,F,G,H
    let expected: Vec<char> = "ABCDEFGH".chars().collect();
    assert_eq!(
        result, expected,
        "Equal stats: input order (A-H) should be preserved"
    );
}

/// Only 8 input entries → all are returned (degenerate case: exactly 12 groups,
/// but we test with fewer for robustness).
#[test]
fn test_best_thirds_fewer_than_12_returns_all() {
    // 10 entries - returns top 8 still
    let thirds: Vec<(char, TeamStats)> = "ABCDEFGHIJ"
        .chars()
        .enumerate()
        .map(|(i, c)| {
            (
                c,
                TeamStats {
                    team_id: c.to_string(),
                    points: (10 - i as i32), // A has highest points
                    goal_diff: 0,
                    goals_for: 1,
                    conduct: 0,
                },
            )
        })
        .collect();
    let result = best_thirds(&thirds);
    assert_eq!(result.len(), 8);
    assert_eq!(result[0], 'A'); // highest points
}
