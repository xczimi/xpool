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
}

impl From<&domain::Team> for Team {
    fn from(t: &domain::Team) -> Self {
        Team {
            id: t.id.clone(),
            name: t.name.clone(),
            short_code: t.short_code.clone(),
            flag: t.flag.clone(),
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
}

impl From<&domain::SingleGame> for Game {
    fn from(g: &domain::SingleGame) -> Self {
        Game {
            id: g.id.clone(),
            kickoff: g.kickoff,
            venue: g.venue.clone(),
            group_id: g.group_id.clone(),
            home: (&g.home).into(),
            away: (&g.away).into(),
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
}

impl From<&domain::GroupGame> for Group {
    fn from(g: &domain::GroupGame) -> Self {
        let (child_group_ids, child_game_ids) = match &g.children {
            domain::GroupChildren::Groups(ids) => (ids.clone(), Vec::new()),
            domain::GroupChildren::Games(ids) => (Vec::new(), ids.clone()),
        };
        Group {
            id: g.id.clone(),
            name: g.name.clone(),
            parent: g.parent.clone(),
            round: g.round.into(),
            lock_mode: g.lock_mode.into(),
            carries_standings: g.carries_standings,
            child_group_ids,
            child_game_ids,
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

impl From<&domain::Tournament> for Tournament {
    fn from(t: &domain::Tournament) -> Self {
        Tournament {
            root: t.root.clone(),
            groups: t.groups.values().map(Group::from).collect(),
            games: t.games.values().map(Game::from).collect(),
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

/// The current player plus their predictions (`me`).
#[derive(SimpleObject, Clone, Debug)]
pub struct Player {
    pub id: String,
    pub nick: String,
    pub full_name: String,
    pub is_result_user: bool,
    pub version: u64,
    pub match_predictions: Vec<MatchPrediction>,
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
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
pub struct Pool {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub members: Vec<String>,
}

impl From<&domain::Pool> for Pool {
    fn from(p: &domain::Pool) -> Self {
        Pool {
            id: p.id.clone(),
            name: p.name.clone(),
            owner: p.owner.clone(),
            members: p.members.clone(),
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
