//! Domain entities. Single-tournament: no `tournament_id` is threaded through
//! these types — tournament scoping is a storage concern (`DATA_MODEL.md` §1–2).
//!
//! This file is a **locked contract** — parallel subsystems depend on it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type TeamId = String;
pub type GroupId = String;
pub type GameId = String;
pub type PlayerId = String;

/// A national team. Per-tournament entity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Team {
    pub id: TeamId,
    pub name: String,
    pub short_code: String,
    pub flag: Option<String>,
    pub external_id: Option<String>,
}

/// A team reference on a match. `team_id` is `None` for an unresolved knockout
/// slot; `description` is the placeholder text (`"2A"`, `"3ABCDF"`,
/// `"Winner SF 1"`) — see `DATA_MODEL.md` §6.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamSlot {
    pub team_id: Option<TeamId>,
    pub description: String,
}

/// One match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SingleGame {
    pub id: GameId,
    pub kickoff: DateTime<Utc>,
    pub venue: Option<String>,
    pub group_id: GroupId,
    pub home: TeamSlot,
    pub away: TeamSlot,
}

/// Tournament round. Drives the scoring multiplier (`SCORING.md` §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Round {
    GroupStage,
    R32,
    R16,
    QF,
    SF,
    ThirdPlace,
    Final,
}

/// Lock granularity of a group node (`DATA_MODEL.md` §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockMode {
    /// Group stage: all predictions in the node lock as a unit.
    LockTogether,
    /// Knockout: each match locks independently.
    LockPerMatch,
}

/// A group node holds either child groups (internal) or matches (leaf group).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupChildren {
    Groups(Vec<GroupId>),
    Games(Vec<GameId>),
}

/// A node in the recursive tournament tree (`DATA_MODEL.md` §4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupGame {
    pub id: GroupId,
    pub name: String,
    pub parent: Option<GroupId>,
    pub round: Round,
    pub lock_mode: LockMode,
    /// Whether this node carries a `StandingsPrediction`.
    pub carries_standings: bool,
    pub children: GroupChildren,
}

/// The whole tournament structure. The domain side of `<t>#TOURNAMENT`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tournament {
    pub root: GroupId,
    pub groups: HashMap<GroupId, GroupGame>,
    pub games: HashMap<GameId, SingleGame>,
    pub teams: HashMap<TeamId, Team>,
}

/// One player's score prediction for one match. Scores are 90-minute scores.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchPrediction {
    pub game_id: GameId,
    pub home_score: u8,
    pub away_score: u8,
    pub locked: bool,
}

/// One player's predicted team ordering for one group node. For a knockout
/// one-match group the 2-team ordering is the ET/penalty advancer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandingsPrediction {
    pub group_id: GroupId,
    /// The predicted final ordering of the node's teams.
    pub ordering: Vec<TeamId>,
    /// Manual tiebreak for everything not score-derivable (`SCORING.md` §4).
    pub draw_order: Vec<TeamId>,
    pub locked: bool,
}

/// A Person's participation in one tournament. The official result is the
/// prediction set of a Player with `is_result_user = true` (`DATA_MODEL.md` §5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub person_id: String,
    pub nick: String,
    pub full_name: String,
    pub referrer: Option<PlayerId>,
    pub is_result_user: bool,
    /// Optimistic-concurrency guard.
    pub version: u64,
    pub match_predictions: Vec<MatchPrediction>,
    pub standings_predictions: Vec<StandingsPrediction>,
}

/// A human. Global (cross-tournament) entity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub id: String,
    pub identity_ids: Vec<String>,
}

/// A login credential. Global entity. `verified_email` is the
/// cross-provider match key — when a login arrives via a new provider and
/// its verified email matches an existing `Person` via this field, AUTH-13
/// linking is triggered (spec §3, §6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub id: String,
    pub provider: String,
    pub provider_id: String,
    pub person_id: String,
    pub verified_email: Option<String>,
}

/// A named subset of players sharing a scoreboard (`DATA_MODEL.md` §8).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pool {
    pub id: String,
    pub name: String,
    pub owner: PlayerId,
    pub members: Vec<PlayerId>,
    /// Opaque, rotatable code that admits a player to the pool
    /// (`SCENARIOS.md` POOL-02). Generated by the application layer.
    pub join_code: String,
}

impl Player {
    /// The player's prediction for one match, if any.
    pub fn match_prediction(&self, game_id: &str) -> Option<&MatchPrediction> {
        self.match_predictions.iter().find(|p| p.game_id == game_id)
    }

    /// The player's standings prediction for one group, if any.
    pub fn standings_prediction(&self, group_id: &str) -> Option<&StandingsPrediction> {
        self.standings_predictions
            .iter()
            .find(|p| p.group_id == group_id)
    }
}

impl Tournament {
    /// Every match in a group node's subtree (recursive).
    pub fn games_in(&self, group_id: &str) -> Vec<&SingleGame> {
        let mut out = Vec::new();
        if let Some(g) = self.groups.get(group_id) {
            match &g.children {
                GroupChildren::Games(ids) => {
                    out.extend(ids.iter().filter_map(|id| self.games.get(id)));
                }
                GroupChildren::Groups(ids) => {
                    for child in ids {
                        out.extend(self.games_in(child));
                    }
                }
            }
        }
        out
    }

    /// The node's deadline — the earliest kickoff in its subtree
    /// (`DATA_MODEL.md` §4).
    pub fn deadline(&self, group_id: &str) -> Option<DateTime<Utc>> {
        self.games_in(group_id).iter().map(|g| g.kickoff).min()
    }
}
