//! GraphQL output types — thin wrappers over `domain` structs.
//!
//! `async-graphql`'s `SimpleObject` cannot be derived on `domain` types
//! (orphan rule — they live in another crate), so each public type gets a
//! minimal local mirror. Conversion is a plain `From` impl; no logic.

use async_graphql::{Enum, SimpleObject};

/// Tournament round — mirrors `domain::Round`.
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum Round {
    GroupStage,
    R32,
    R16,
    Qf,
    Sf,
    ThirdPlace,
    Final,
}

impl From<domain::Round> for Round {
    fn from(r: domain::Round) -> Self {
        match r {
            domain::Round::GroupStage => Round::GroupStage,
            domain::Round::R32 => Round::R32,
            domain::Round::R16 => Round::R16,
            domain::Round::QF => Round::Qf,
            domain::Round::SF => Round::Sf,
            domain::Round::ThirdPlace => Round::ThirdPlace,
            domain::Round::Final => Round::Final,
        }
    }
}

/// Lock granularity — mirrors `domain::LockMode`.
#[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum LockMode {
    LockTogether,
    LockPerMatch,
}

impl From<domain::LockMode> for LockMode {
    fn from(m: domain::LockMode) -> Self {
        match m {
            domain::LockMode::LockTogether => LockMode::LockTogether,
            domain::LockMode::LockPerMatch => LockMode::LockPerMatch,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub short_code: String,
    pub flag: Option<String>,
    /// The id of the team in the upstream data source, if any.
    pub external_id: Option<String>,
}

impl From<&domain::Team> for Team {
    fn from(t: &domain::Team) -> Self {
        Team {
            id: t.id.clone(),
            name: t.name.clone(),
            short_code: t.short_code.clone(),
            flag: t.flag.clone(),
            external_id: t.external_id.clone(),
        }
    }
}

/// A team reference on a match. `team_id` is `None` for an unresolved
/// knockout slot; `description` is the placeholder text.
#[derive(SimpleObject, Clone, Debug)]
pub struct TeamSlot {
    pub team_id: Option<String>,
    pub description: String,
}

impl From<&domain::TeamSlot> for TeamSlot {
    fn from(s: &domain::TeamSlot) -> Self {
        TeamSlot {
            team_id: s.team_id.clone(),
            description: s.description.clone(),
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
pub struct Game {
    pub id: String,
    pub kickoff: chrono::DateTime<chrono::Utc>,
    pub venue: Option<String>,
    pub group_id: String,
    pub home: TeamSlot,
    pub away: TeamSlot,
    pub result_pending: bool,
    pub within_today_window: bool,
}

impl Game {
    fn build(
        g: &domain::SingleGame,
        round: domain::Round,
        now: chrono::DateTime<chrono::Utc>,
        locked_result_game_ids: &std::collections::HashSet<String>,
    ) -> Self {
        Game {
            id: g.id.clone(),
            kickoff: g.kickoff,
            venue: g.venue.clone(),
            group_id: g.group_id.clone(),
            home: (&g.home).into(),
            away: (&g.away).into(),
            result_pending: crate::timeflags::result_pending(
                g.kickoff,
                round,
                locked_result_game_ids.contains(&g.id),
                now,
            ),
            within_today_window: crate::timeflags::within_today_window(g.kickoff, now),
        }
    }
}

/// One node in the tournament tree. `childGroupIds` / `childGameIds` carry the
/// children depending on the node kind (exactly one is populated).
#[derive(SimpleObject, Clone, Debug)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
    pub round: Round,
    pub lock_mode: LockMode,
    pub carries_standings: bool,
    pub child_group_ids: Vec<String>,
    pub child_game_ids: Vec<String>,
    /// The earliest kickoff in this node's subtree, if it has any games.
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
    pub deadline_passed: bool,
}

impl Group {
    /// Build a `Group` from a tournament node, computing the subtree deadline
    /// from the full tournament (the deadline is not stored on the node).
    fn build(
        g: &domain::GroupGame,
        tournament: &domain::Tournament,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        let (child_group_ids, child_game_ids) = match &g.children {
            domain::GroupChildren::Groups(ids) => (ids.clone(), Vec::new()),
            domain::GroupChildren::Games(ids) => (Vec::new(), ids.clone()),
        };
        let deadline = tournament.deadline(&g.id);
        Group {
            id: g.id.clone(),
            name: g.name.clone(),
            parent: g.parent.clone(),
            round: g.round.into(),
            lock_mode: g.lock_mode.into(),
            carries_standings: g.carries_standings,
            child_group_ids,
            child_game_ids,
            deadline,
            deadline_passed: crate::timeflags::deadline_passed(deadline, now),
        }
    }
}

/// The whole tournament structure (tree + matches + teams).
#[derive(SimpleObject, Clone, Debug)]
pub struct Tournament {
    pub root: String,
    pub groups: Vec<Group>,
    pub games: Vec<Game>,
    pub teams: Vec<Team>,
}

impl Tournament {
    pub fn build(
        t: &domain::Tournament,
        now: chrono::DateTime<chrono::Utc>,
        locked_result_game_ids: &std::collections::HashSet<String>,
    ) -> Self {
        Tournament {
            root: t.root.clone(),
            groups: t.groups.values().map(|g| Group::build(g, t, now)).collect(),
            games: t
                .games
                .values()
                .map(|g| {
                    let round = t
                        .groups
                        .get(&g.group_id)
                        .map(|grp| grp.round)
                        .unwrap_or(domain::Round::GroupStage);
                    Game::build(g, round, now, locked_result_game_ids)
                })
                .collect(),
            teams: t.teams.values().map(Team::from).collect(),
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
pub struct MatchPrediction {
    pub game_id: String,
    pub home_score: i32,
    pub away_score: i32,
    pub locked: bool,
}

impl From<&domain::MatchPrediction> for MatchPrediction {
    fn from(p: &domain::MatchPrediction) -> Self {
        MatchPrediction {
            game_id: p.game_id.clone(),
            home_score: p.home_score as i32,
            away_score: p.away_score as i32,
            locked: p.locked,
        }
    }
}

/// One player's predicted ordering for one group node.
#[derive(SimpleObject, Clone, Debug)]
pub struct StandingsPrediction {
    pub group_id: String,
    /// The predicted final ordering of the node's teams.
    pub ordering: Vec<String>,
    /// Manual tiebreak for everything not score-derivable.
    pub draw_order: Vec<String>,
    pub locked: bool,
}

impl From<&domain::StandingsPrediction> for StandingsPrediction {
    fn from(s: &domain::StandingsPrediction) -> Self {
        StandingsPrediction {
            group_id: s.group_id.clone(),
            ordering: s.ordering.clone(),
            draw_order: s.draw_order.clone(),
            locked: s.locked,
        }
    }
}

/// The current player plus their predictions (`me`).
#[derive(SimpleObject, Clone, Debug)]
pub struct Player {
    pub id: String,
    pub nick: String,
    pub full_name: String,
    pub is_result_user: bool,
    pub version: u64,
    pub match_predictions: Vec<MatchPrediction>,
    pub standings_predictions: Vec<StandingsPrediction>,
}

impl From<&domain::Player> for Player {
    fn from(p: &domain::Player) -> Self {
        Player {
            id: p.id.clone(),
            nick: p.nick.clone(),
            full_name: p.full_name.clone(),
            is_result_user: p.is_result_user,
            version: p.version,
            match_predictions: p
                .match_predictions
                .iter()
                .map(MatchPrediction::from)
                .collect(),
            standings_predictions: p
                .standings_predictions
                .iter()
                .map(StandingsPrediction::from)
                .collect(),
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
pub struct Pool {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub members: Vec<String>,
    /// The code that admits a player to this pool. Visible to members only —
    /// the `pools` query already returns a pool solely to its owner/members.
    pub join_code: String,
}

impl From<&domain::Pool> for Pool {
    fn from(p: &domain::Pool) -> Self {
        Pool {
            id: p.id.clone(),
            name: p.name.clone(),
            owner: p.owner.clone(),
            members: p.members.clone(),
            join_code: p.join_code.clone(),
        }
    }
}

/// One row of the materialised scoreboard for one player.
#[derive(SimpleObject, Clone, Debug)]
pub struct ScoreEntry {
    pub player_id: String,
    pub nick: String,
    /// Sum across all rounds.
    pub total: i64,
    pub stages: Vec<StageScore>,
}

#[derive(SimpleObject, Clone, Debug)]
pub struct StageScore {
    pub round: Round,
    pub points: i64,
}

/// One visible tip in the `tips(groupId)` grid.
#[derive(SimpleObject, Clone, Debug)]
pub struct Tip {
    pub player_id: String,
    pub nick: String,
    pub game_id: String,
    /// `None` when the player's prediction is still hidden from others.
    pub prediction: Option<MatchPrediction>,
}

/// One perfect prediction (`perfects`).
#[derive(SimpleObject, Clone, Debug)]
pub struct Perfect {
    pub player_id: String,
    pub nick: String,
    pub game_id: String,
}

/// A lightweight player listing — for the dev-login picker and the admin
/// player list (UC-16). No predictions.
#[derive(SimpleObject, Clone, Debug)]
pub struct PlayerSummary {
    pub id: String,
    pub nick: String,
    pub full_name: String,
    pub is_result_user: bool,
}

impl From<&domain::Player> for PlayerSummary {
    fn from(p: &domain::Player) -> Self {
        PlayerSummary {
            id: p.id.clone(),
            nick: p.nick.clone(),
            full_name: p.full_name.clone(),
            is_result_user: p.is_result_user,
        }
    }
}
