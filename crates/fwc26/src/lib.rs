//! FIFA World Cup 26 specific logic — kept out of the generic `domain` model
//! (`DATA_MODEL.md` §6, `DATA_SOURCES.md` §5).
//!
//! Signatures here are a **locked contract**. `todo!()` bodies and the Annexe C
//! data table are filled by the `fwc26` subagent (plan task P2).

use domain::{GameId, Player, TeamId, Tournament};
use std::collections::{BTreeSet, HashMap};

/// Aggregated group-stage stats for one team — input to the third-placed
/// ranking (`FWC26_RULES.md` §3).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TeamStats {
    pub team_id: TeamId,
    pub points: i32,
    pub goal_diff: i32,
    pub goals_for: i32,
    pub conduct: i32,
}

/// Annexe C lookup (`FWC26_RULES.md` §5): given the set of 8 group letters whose
/// third-placed team qualified, return the mapping `winner-group → third-group`
/// for the 8 winners `{A,B,D,E,G,I,K,L}`. `None` if the set is not a valid
/// 8-of-12 combination.
pub fn annexe_c(_qualifying_third_groups: &BTreeSet<char>) -> Option<HashMap<char, char>> {
    todo!("P2: embed and look up the 495-row Annexe C table")
}

/// Rank the 12 third-placed teams (`FWC26_RULES.md` §3); return the group
/// letters of the top 8, best first.
pub fn best_thirds(_thirds: &[(char, TeamStats)]) -> Vec<char> {
    todo!("P2: implement per FWC26_RULES.md §3")
}

/// Resolve every knockout `TeamSlot` description to a concrete team given the
/// current official results. Pure (`DATA_SOURCES.md` §5). Slots not yet
/// determinable stay `None`; self-correcting on result changes.
pub fn resolve_bracket(
    _t: &Tournament,
    _result: &Player,
) -> HashMap<GameId, (Option<TeamId>, Option<TeamId>)> {
    todo!("P2: implement per FWC26_RULES.md §4-5, DATA_SOURCES.md §5")
}
