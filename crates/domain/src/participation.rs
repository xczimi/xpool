//! Participation predicates — "did this player enter the relevant predictions"
//! (exclude-non-participating-players design, 2026-06-14).
//!
//! Pure, I/O-free selectors the thin resolvers delegate to, the same way they
//! delegate scoring (`scoring.rs`) and pool rules (`pool.rs`). They answer a
//! domain question (predictions exist), never a presentation one (visibility,
//! points), so the same call returns the same answer for any client — the test
//! that this is domain logic, not view-coupling. The result-user is folded into
//! all of them: it is never a competitor in any listing.

use crate::{GameId, GroupId, Player};

impl Player {
    /// A competing player who has entered at least one prediction.
    /// False for the result-user and for players with no predictions at all.
    pub fn is_participant(&self) -> bool {
        !self.is_result_user
            && (!self.match_predictions.is_empty() || !self.standings_predictions.is_empty())
    }
}

/// Competitors for global listings (Scoreboard): participants only.
pub fn participants(players: &[Player]) -> Vec<&Player> {
    players.iter().filter(|p| p.is_participant()).collect()
}

/// Players with at least one match prediction among `game_ids` (All Tips).
/// Excludes the result-user.
pub fn tippers_in<'a>(players: &'a [Player], game_ids: &[GameId]) -> Vec<&'a Player> {
    players
        .iter()
        .filter(|p| !p.is_result_user)
        .filter(|p| game_ids.iter().any(|g| p.match_prediction(g).is_some()))
        .collect()
}

/// Players with at least one standings prediction among `group_ids`
/// (Standings-bonus grid). Excludes the result-user.
pub fn standings_tippers<'a>(players: &'a [Player], group_ids: &[GroupId]) -> Vec<&'a Player> {
    players
        .iter()
        .filter(|p| !p.is_result_user)
        .filter(|p| {
            group_ids
                .iter()
                .any(|g| p.standings_prediction(g).is_some())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MatchPrediction, StandingsPrediction};

    /// Build a player with one match prediction per id in `games` and one
    /// standings prediction per id in `groups`.
    fn mk(id: &str, is_result: bool, games: &[&str], groups: &[&str]) -> Player {
        Player {
            id: id.into(),
            person_id: id.into(),
            nick: id.into(),
            full_name: id.into(),
            referrer: None,
            is_result_user: is_result,
            version: 0,
            match_predictions: games
                .iter()
                .map(|g| MatchPrediction {
                    game_id: (*g).into(),
                    home_score: 1,
                    away_score: 0,
                    locked: true,
                })
                .collect(),
            standings_predictions: groups
                .iter()
                .map(|g| StandingsPrediction {
                    group_id: (*g).into(),
                    ordering: vec![],
                    draw_order: vec![],
                    locked: true,
                })
                .collect(),
        }
    }

    #[test]
    fn is_participant_truth_table() {
        assert!(
            !mk("ru", true, &["M1"], &["A"]).is_participant(),
            "result-user never participates, even with predictions"
        );
        assert!(
            !mk("empty", false, &[], &[]).is_participant(),
            "no predictions → not a participant"
        );
        assert!(
            mk("matchonly", false, &["M1"], &[]).is_participant(),
            "a single match tip is enough"
        );
        assert!(
            mk("standingsonly", false, &[], &["A"]).is_participant(),
            "a single standings tip is enough"
        );
    }

    #[test]
    fn participants_keeps_only_participating_competitors() {
        let players = vec![
            mk("ru", true, &["M1"], &["A"]),
            mk("empty", false, &[], &[]),
            mk("ada", false, &["M1"], &[]),
        ];
        let got: Vec<&str> = participants(&players)
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(got, vec!["ada"]);
    }

    #[test]
    fn participants_handles_empty_input() {
        assert!(participants(&[]).is_empty());
    }

    #[test]
    fn tippers_in_selects_match_tippers_and_excludes_result_user() {
        let players = vec![
            mk("ru", true, &["M1"], &[]),    // result-user → excluded
            mk("ada", false, &["M1"], &[]),  // tipped M1 → in
            mk("alan", false, &["M2"], &[]), // tipped M2, not in [M1] → out
            mk("stand", false, &[], &["A"]), // standings only → out of match grid
        ];
        let got: Vec<&str> = tippers_in(&players, &["M1".to_string()])
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(got, vec!["ada"]);
    }

    #[test]
    fn tippers_in_handles_empty_games() {
        let players = vec![mk("ada", false, &["M1"], &[])];
        assert!(tippers_in(&players, &[]).is_empty());
    }

    #[test]
    fn standings_tippers_selects_standings_tippers_and_excludes_result_user() {
        let players = vec![
            mk("ru", true, &[], &["A"]),      // result-user → excluded
            mk("ada", false, &[], &["A"]),    // standings A → in
            mk("alan", false, &[], &["B"]),   // standings B, not in [A] → out
            mk("match", false, &["M1"], &[]), // match only → out of standings grid
        ];
        let got: Vec<&str> = standings_tippers(&players, &["A".to_string()])
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(got, vec!["ada"]);
    }

    #[test]
    fn standings_tippers_handles_empty_groups() {
        let players = vec![mk("ada", false, &[], &["A"])];
        assert!(standings_tippers(&players, &[]).is_empty());
    }
}
