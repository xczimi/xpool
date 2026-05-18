//! FIFA World Cup 26 specific logic — kept out of the generic `domain` model
//! (`DATA_MODEL.md` §6, `DATA_SOURCES.md` §5).
//!
//! Signatures here are a **locked contract**. `todo!()` bodies and the Annexe C
//! data table are filled by the `fwc26` subagent (plan task P2).

use domain::{GameId, GroupChildren, MatchPrediction, Player, Round, TeamId, Tournament};
use std::collections::{BTreeSet, HashMap};

mod annexe_c_data;

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

/// The 8 group-winner columns in the Annexe C table, in order.
/// These are the only winners that face a 3rd-placed team in R32.
const THIRD_FACING_WINNERS: [char; 8] = ['A', 'B', 'D', 'E', 'G', 'I', 'K', 'L'];

/// Annexe C lookup (`FWC26_RULES.md` §5): given the set of 8 group letters whose
/// third-placed team qualified, return the mapping `winner-group → third-group`
/// for the 8 winners `{A,B,D,E,G,I,K,L}`. `None` if the set is not a valid
/// 8-of-12 combination.
pub fn annexe_c(qualifying_third_groups: &BTreeSet<char>) -> Option<HashMap<char, char>> {
    if qualifying_third_groups.len() != 8 {
        return None;
    }

    // Build the lookup key as a sorted [u8; 8]
    let key: Vec<u8> = qualifying_third_groups.iter().map(|&c| c as u8).collect();
    let key_arr: [u8; 8] = key.try_into().ok()?;

    // Search the table
    for (qualifying, thirds) in annexe_c_data::ANNEXE_C {
        if qualifying == &key_arr {
            // Build winner -> third mapping
            let mut map = HashMap::new();
            for (i, &winner) in THIRD_FACING_WINNERS.iter().enumerate() {
                map.insert(winner, thirds[i] as char);
            }
            return Some(map);
        }
    }
    None
}

/// Rank the 12 third-placed teams (`FWC26_RULES.md` §3); return the group
/// letters of the top 8, best first.
///
/// Criteria applied in order (§3 a-d, then input order for the rest):
///   a. Most points
///   b. Superior goal difference
///   c. Greatest goals scored
///   d. Highest team conduct score (less negative = better)
///
/// Criteria e/f (FIFA ranking) are not score-derivable and require external
/// data. If still tied after a-d, we preserve input order (stable sort).
/// This is documented as a known limitation: ties after conduct are broken
/// by input order, not FIFA ranking.
pub fn best_thirds(thirds: &[(char, TeamStats)]) -> Vec<char> {
    let mut indexed: Vec<(usize, char, &TeamStats)> = thirds
        .iter()
        .enumerate()
        .map(|(i, (g, s))| (i, *g, s))
        .collect();

    // Stable sort so equal teams preserve input order (stand-in for FIFA ranking)
    indexed.sort_by(|a, b| {
        b.2.points
            .cmp(&a.2.points)
            .then_with(|| b.2.goal_diff.cmp(&a.2.goal_diff))
            .then_with(|| b.2.goals_for.cmp(&a.2.goals_for))
            .then_with(|| b.2.conduct.cmp(&a.2.conduct))
            .then_with(|| a.0.cmp(&b.0)) // stable: preserve input order
    });

    indexed.iter().take(8).map(|(_, g, _)| *g).collect()
}

// ---------------------------------------------------------------------------
// Bracket resolution
// ---------------------------------------------------------------------------

/// Resolve every knockout `TeamSlot` description to a concrete team given the
/// current official results. Pure (`DATA_SOURCES.md` §5). Slots not yet
/// determinable stay `None`; self-correcting on result changes.
///
/// Slot description grammar (from `DATA_MODEL.md` §6 and the FotMob ICS):
///   - `"1X"` — winner of group X
///   - `"2X"` — runner-up of group X
///   - `"3ABCDF"` — best 3rd-placed team from the listed groups (resolved via
///     Annexe C once 8 qualifying thirds are known)
///   - `"Winner MNN"` / `"Winner SF N"` — winner of match NN
///   - `"Loser MNN"` / `"Loser SF N"` — loser of match NN (used for 3rd-place)
pub fn resolve_bracket(
    t: &Tournament,
    result: &Player,
) -> HashMap<GameId, (Option<TeamId>, Option<TeamId>)> {
    let ctx = ResolutionContext::build(t, result);

    let mut out = HashMap::new();
    for (game_id, game) in &t.games {
        let home = ctx.resolve_slot(&game.home.description);
        let away = ctx.resolve_slot(&game.away.description);
        out.insert(game_id.clone(), (home, away));
    }
    out
}

// ---------------------------------------------------------------------------
// Resolution context — internal helpers
// ---------------------------------------------------------------------------

/// Per-group computed standings (ordered team ids, 1st to last).
struct GroupStandings {
    /// Ordered team ids, 1st first.
    order: Vec<TeamId>,
}

struct ResolutionContext {
    /// group letter (A-L) -> ordered standings
    group_standings: HashMap<char, GroupStandings>,
    /// game_id -> winning team_id (if known)
    knockout_winners: HashMap<GameId, TeamId>,
    /// game_id -> losing team_id (if known)
    knockout_losers: HashMap<GameId, TeamId>,
    /// Annexe C resolved map: winner-group -> third-group (if all 8 thirds qualified)
    annexe_c_map: Option<HashMap<char, char>>,
    /// group letter -> 3rd-placed team_id (only qualifying thirds)
    qualifying_thirds: HashMap<char, TeamId>,
}

impl ResolutionContext {
    fn build(t: &Tournament, result: &Player) -> Self {
        // 1. Compute group-stage standings for every group A-L
        let group_standings = compute_group_standings(t, result);

        // 2. Determine qualifying 3rd-placed teams (top 8) and build Annexe C map
        let mut thirds_input: Vec<(char, TeamStats)> = Vec::new();
        for letter in 'A'..='L' {
            if let Some(gs) = group_standings.get(&letter) {
                if gs.order.len() >= 3 {
                    let third_id = gs.order[2].clone();
                    // We need stats for the third-placed team.
                    // Recompute from games.
                    let stats = compute_team_stats_in_group(t, result, letter, &third_id);
                    thirds_input.push((letter, stats));
                }
            }
        }

        let qualifying_letters: Vec<char> = best_thirds(&thirds_input);
        let qualifying_set: BTreeSet<char> = qualifying_letters.iter().copied().collect();

        let mut qualifying_thirds: HashMap<char, TeamId> = HashMap::new();
        for &letter in &qualifying_letters {
            if let Some(gs) = group_standings.get(&letter) {
                if gs.order.len() >= 3 {
                    qualifying_thirds.insert(letter, gs.order[2].clone());
                }
            }
        }

        let annexe_c_map = if qualifying_set.len() == 8 {
            annexe_c(&qualifying_set)
        } else {
            None
        };

        // 3. Resolve knockout winners/losers iteratively
        // We do multiple passes because later matches depend on earlier ones.
        let mut knockout_winners: HashMap<GameId, TeamId> = HashMap::new();
        let mut knockout_losers: HashMap<GameId, TeamId> = HashMap::new();

        // Collect all knockout games in round order
        let knockout_games = collect_knockout_games_in_order(t);

        // Build temporary slot resolver (without knockout_winners yet)
        // We iterate up to N passes to handle chained dependencies
        for _pass in 0..10 {
            let changed = resolve_knockout_pass(
                t,
                result,
                &group_standings,
                &qualifying_thirds,
                &annexe_c_map,
                &knockout_games,
                &mut knockout_winners,
                &mut knockout_losers,
            );
            if !changed {
                break;
            }
        }

        ResolutionContext {
            group_standings,
            knockout_winners,
            knockout_losers,
            annexe_c_map,
            qualifying_thirds,
        }
    }

    fn resolve_slot(&self, desc: &str) -> Option<TeamId> {
        let desc = desc.trim();
        if desc.is_empty() {
            return None;
        }

        // "1X" — winner of group X
        if let Some(letter) = parse_group_position(desc, '1') {
            return self
                .group_standings
                .get(&letter)
                .and_then(|gs| gs.order.first().cloned());
        }

        // "2X" — runner-up of group X
        if let Some(letter) = parse_group_position(desc, '2') {
            return self
                .group_standings
                .get(&letter)
                .and_then(|gs| gs.order.get(1).cloned());
        }

        // "3ABCDF" — best 3rd from listed groups, resolved via Annexe C
        if let Some(groups_str) = desc.strip_prefix('3') {
            if groups_str.chars().all(|c| c.is_ascii_uppercase()) && groups_str.len() > 1 {
                return self.resolve_best_third_from(groups_str);
            }
            // "3X" — 3rd-placed team of group X (direct reference)
            if groups_str.len() == 1 {
                let letter = groups_str.chars().next().unwrap();
                return self.qualifying_thirds.get(&letter).cloned();
            }
        }

        // "Winner MNN" or "Winner SF N" or "Winner M73"
        if let Some(game_id) = parse_match_ref(desc, "Winner") {
            return self.knockout_winners.get(&game_id).cloned();
        }

        // "Loser MNN" or "Loser SF N" or "Loser M101"
        if let Some(game_id) = parse_match_ref(desc, "Loser") {
            return self.knockout_losers.get(&game_id).cloned();
        }

        None
    }

    /// Resolve "3ABCDF" type descriptions: best third from a subset of groups.
    /// Uses Annexe C map (which winner gets which third).
    fn resolve_best_third_from(&self, groups_str: &str) -> Option<TeamId> {
        let annex = self.annexe_c_map.as_ref()?;

        // The groups_str tells us which groups contribute thirds in this slot.
        // The Annexe C map tells us: for each winner group that faces a third,
        // which third group it faces. We need to find which of the listed groups
        // is assigned to the match that needs this slot.
        //
        // Actually, "3ABCDF" means: the best 3rd-placed team from groups A,B,C,D,F
        // *as determined by Annexe C*. Annexe C assigns one specific third-group letter
        // to each winner slot. So we need to know WHICH winner slot is asking for this.
        //
        // However, `resolve_slot` doesn't have context about which match is being
        // resolved. The description "3ABCDF" fully specifies the match slot from
        // FWC26_RULES §4. Each such description appears in exactly one match slot.
        //
        // The mapping from "groups_str" to the winner group is fixed per §4:
        //   M74 (1E): 3ABCDF  → winner=E facing 3rd from {A,B,C,D,F}
        //   M77 (1I): 3CDFGH  → winner=I facing 3rd from {C,D,F,G,H}
        //   M79 (1A): 3CEFHI  → winner=A facing 3rd from {C,E,F,H,I}
        //   M80 (1L): 3EHIJK  → winner=L facing 3rd from {E,H,I,J,K}
        //   M81 (1D): 3BEFIJ  → winner=D facing 3rd from {B,E,F,I,J}
        //   M82 (1G): 3AEHIJ  → winner=G facing 3rd from {A,E,H,I,J}
        //   M85 (1B): 3EFGIJ  → winner=B facing 3rd from {E,F,G,I,J}
        //   M87 (1K): 3DEIJL  → winner=K facing 3rd from {D,E,I,J,L}
        let winner_group = match groups_str {
            "ABCDF" => 'E',
            "CDFGH" => 'I',
            "CEFHI" => 'A',
            "EHIJK" => 'L',
            "BEFIJ" => 'D',
            "AEHIJ" => 'G',
            "EFGIJ" => 'B',
            "DEIJL" => 'K',
            _ => return None,
        };

        // Look up which third-group is assigned to this winner
        let third_group = annex.get(&winner_group)?;
        // Return the third-placed team from that group
        self.qualifying_thirds.get(third_group).cloned()
    }
}

// ---------------------------------------------------------------------------
// Helpers: group standings computation
// ---------------------------------------------------------------------------

/// Compute group standings for all groups A-L from the result player's predictions.
fn compute_group_standings(t: &Tournament, result: &Player) -> HashMap<char, GroupStandings> {
    let mut out = HashMap::new();

    for letter in 'A'..='L' {
        if let Some(gs) = compute_standings_for_group(t, result, letter) {
            out.insert(letter, gs);
        }
    }
    out
}

/// Find all group-stage games for group `letter`, compute standings.
fn compute_standings_for_group(
    t: &Tournament,
    result: &Player,
    letter: char,
) -> Option<GroupStandings> {
    // Find the group-stage group node for this letter
    let group_id = find_group_id(t, letter)?;
    let games = t.games_in(&group_id);

    if games.is_empty() {
        return None;
    }

    // Collect all teams in this group
    let mut team_ids: Vec<TeamId> = Vec::new();
    for game in &games {
        if let Some(ref tid) = game.home.team_id {
            if !team_ids.contains(tid) {
                team_ids.push(tid.clone());
            }
        }
        if let Some(ref tid) = game.away.team_id {
            if !team_ids.contains(tid) {
                team_ids.push(tid.clone());
            }
        }
    }

    if team_ids.is_empty() {
        return None;
    }

    // Compute raw stats per team
    let mut stats: HashMap<TeamId, RawStats> = team_ids
        .iter()
        .map(|id| (id.clone(), RawStats::default()))
        .collect();

    for game in &games {
        let pred = result.match_prediction(&game.id)?;
        let home_id = game.home.team_id.as_ref()?;
        let away_id = game.away.team_id.as_ref()?;

        let h = pred.home_score as i32;
        let a = pred.away_score as i32;

        let home_stats = stats.entry(home_id.clone()).or_default();
        home_stats.goals_for += h;
        home_stats.goals_against += a;
        if h > a {
            home_stats.points += 3;
        } else if h == a {
            home_stats.points += 1;
        }

        let away_stats = stats.entry(away_id.clone()).or_default();
        away_stats.goals_for += a;
        away_stats.goals_against += h;
        if a > h {
            away_stats.points += 3;
        } else if h == a {
            away_stats.points += 1;
        }
    }

    // Get draw_order from standings prediction (for tiebreaking)
    let draw_order: Vec<TeamId> = result
        .standings_prediction(&group_id)
        .map(|sp| sp.draw_order.clone())
        .unwrap_or_default();

    // Sort teams by standings ladder (simplified: points > GD > GF > draw_order)
    // Full tiebreaker (§2) requires head-to-head; we implement a simplified version
    // sufficient for bracket resolution: points, then GD, then GF, then draw_order.
    let mut ordered: Vec<(TeamId, RawStats)> = stats.into_iter().collect();
    ordered.sort_by(|a, b| {
        let a_gd = a.1.goals_for - a.1.goals_against;
        let b_gd = b.1.goals_for - b.1.goals_against;
        b.1.points
            .cmp(&a.1.points)
            .then_with(|| b_gd.cmp(&a_gd))
            .then_with(|| b.1.goals_for.cmp(&a.1.goals_for))
            .then_with(|| {
                // Use draw_order position for final tiebreak
                let a_pos = draw_order
                    .iter()
                    .position(|x| x == &a.0)
                    .unwrap_or(usize::MAX);
                let b_pos = draw_order
                    .iter()
                    .position(|x| x == &b.0)
                    .unwrap_or(usize::MAX);
                a_pos.cmp(&b_pos)
            })
    });

    Some(GroupStandings {
        order: ordered.into_iter().map(|(id, _)| id).collect(),
    })
}

/// Compute `TeamStats` for a specific team in a specific group (for `best_thirds`).
fn compute_team_stats_in_group(
    t: &Tournament,
    result: &Player,
    letter: char,
    team_id: &TeamId,
) -> TeamStats {
    let group_id = match find_group_id(t, letter) {
        Some(id) => id,
        None => return TeamStats::default(),
    };
    let games = t.games_in(&group_id);

    let mut points = 0i32;
    let mut goals_for = 0i32;
    let mut goals_against = 0i32;

    for game in &games {
        let is_home = game.home.team_id.as_deref() == Some(team_id.as_str());
        let is_away = game.away.team_id.as_deref() == Some(team_id.as_str());
        if !is_home && !is_away {
            continue;
        }

        let pred = match result.match_prediction(&game.id) {
            Some(p) => p,
            None => continue,
        };

        let h = pred.home_score as i32;
        let a = pred.away_score as i32;

        if is_home {
            goals_for += h;
            goals_against += a;
            if h > a {
                points += 3;
            } else if h == a {
                points += 1;
            }
        } else {
            goals_for += a;
            goals_against += h;
            if a > h {
                points += 3;
            } else if h == a {
                points += 1;
            }
        }
    }

    TeamStats {
        team_id: team_id.clone(),
        points,
        goal_diff: goals_for - goals_against,
        goals_for,
        conduct: 0, // conduct data not available from match predictions alone
    }
}

/// Find the GroupGame id for a group-stage group by its letter (e.g. 'A' → "group-A").
/// We search for a group whose name contains the letter and is GroupStage round with
/// Games children (leaf group).
fn find_group_id(t: &Tournament, letter: char) -> Option<String> {
    // Look for a GroupGame of round GroupStage with a Games children and name matching letter
    for (id, group) in &t.groups {
        if group.round == Round::GroupStage {
            if let GroupChildren::Games(_) = &group.children {
                // Check if the name or id suggests this group letter
                let name_upper = group.name.to_uppercase();
                let id_upper = id.to_uppercase();
                // Common patterns: "Group A", "group-a", "A", "group_a"
                if name_upper == format!("GROUP {}", letter)
                    || name_upper == format!("GROUP {}", letter).to_uppercase()
                    || name_upper == letter.to_string()
                    || id_upper == format!("GROUP-{}", letter)
                    || id_upper == format!("GROUP_{}", letter)
                    || id_upper == format!("GROUP{}", letter)
                    || id_upper == letter.to_string()
                    || id == &format!("group-{}", letter.to_lowercase().next().unwrap_or(letter))
                    || id == &format!("group_{}", letter.to_lowercase().next().unwrap_or(letter))
                    || id == &format!("group{}", letter.to_lowercase().next().unwrap_or(letter))
                    || id == &format!("group-{}", letter)
                    || id == &format!("group_{}", letter)
                    || id == &format!("group{}", letter)
                    || id == &format!("G{}", letter)
                    || *id == letter.to_string()
                {
                    return Some(id.clone());
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Helpers: knockout resolution
// ---------------------------------------------------------------------------

#[derive(Default)]
struct RawStats {
    points: i32,
    goals_for: i32,
    goals_against: i32,
}

/// Collect knockout game ids in match-number order (M73 first, M104 last).
fn collect_knockout_games_in_order(t: &Tournament) -> Vec<GameId> {
    // Extract match number from game id for sorting
    let mut games: Vec<(u32, GameId)> = t
        .games
        .keys()
        .filter_map(|id| {
            let n = parse_match_number(id)?;
            if n >= 73 {
                Some((n, id.clone()))
            } else {
                None
            }
        })
        .collect();
    games.sort_by_key(|(n, _)| *n);
    games.into_iter().map(|(_, id)| id).collect()
}

/// Parse a match number from a game id like "M73", "m73", "73".
fn parse_match_number(id: &str) -> Option<u32> {
    let digits = id.trim_start_matches(|c: char| !c.is_ascii_digit());
    digits.parse::<u32>().ok()
}

/// One pass over knockout games: try to resolve winners/losers.
/// Returns true if any new resolution was made.
#[allow(clippy::too_many_arguments)]
fn resolve_knockout_pass(
    t: &Tournament,
    result: &Player,
    group_standings: &HashMap<char, GroupStandings>,
    qualifying_thirds: &HashMap<char, TeamId>,
    annexe_c_map: &Option<HashMap<char, char>>,
    knockout_games: &[GameId],
    knockout_winners: &mut HashMap<GameId, TeamId>,
    knockout_losers: &mut HashMap<GameId, TeamId>,
) -> bool {
    let mut changed = false;

    for game_id in knockout_games {
        // Skip if already resolved
        if knockout_winners.contains_key(game_id) {
            continue;
        }

        let game = match t.games.get(game_id) {
            Some(g) => g,
            None => continue,
        };

        // Try to resolve home and away team
        let home = resolve_description(
            &game.home.description,
            group_standings,
            qualifying_thirds,
            annexe_c_map,
            knockout_winners,
            knockout_losers,
        );
        let away = resolve_description(
            &game.away.description,
            group_standings,
            qualifying_thirds,
            annexe_c_map,
            knockout_winners,
            knockout_losers,
        );

        // If both teams are known and we have a result, determine winner/loser
        if let (Some(home_id), Some(away_id)) = (home, away) {
            if let Some(pred) = result.match_prediction(game_id) {
                let (winner, loser) =
                    determine_winner_loser(pred, &home_id, &away_id, &game.group_id, result);
                knockout_winners.insert(game_id.clone(), winner);
                knockout_losers.insert(game_id.clone(), loser);
                changed = true;
            }
        }
    }

    changed
}

/// Determine winner and loser from a knockout match prediction.
///
/// Knockout matches cannot end drawn: when the 90-minute score is level the
/// advancer is decided in extra time / penalties. The result user encodes that
/// advancer in the `StandingsPrediction` of the match's one-match knockout
/// group — `ordering[0]` is the advancer (`DATA_MODEL.md` §6). This mirrors the
/// group-stage tie path, which consults the same standings prediction for
/// tiebreaking.
///
/// `group_id` is the id of the match's wrapping one-match group. If the level
/// match has no usable standings prediction the home team advances as a
/// last-resort fallback.
fn determine_winner_loser(
    pred: &MatchPrediction,
    home_id: &TeamId,
    away_id: &TeamId,
    group_id: &str,
    result: &Player,
) -> (TeamId, TeamId) {
    if pred.home_score > pred.away_score {
        (home_id.clone(), away_id.clone())
    } else if pred.away_score > pred.home_score {
        (away_id.clone(), home_id.clone())
    } else {
        // Level after 90 minutes → resolve the ET/penalty advancer from the
        // result user's standings prediction for this one-match group.
        let advancer = result
            .standings_prediction(group_id)
            .and_then(|sp| sp.ordering.first())
            .filter(|first| *first == home_id || *first == away_id);
        match advancer {
            Some(first) if first == away_id => (away_id.clone(), home_id.clone()),
            // first == home_id, or no usable prediction → home advances.
            _ => (home_id.clone(), away_id.clone()),
        }
    }
}

/// Resolve a single slot description within a knockout resolution pass.
fn resolve_description(
    desc: &str,
    group_standings: &HashMap<char, GroupStandings>,
    qualifying_thirds: &HashMap<char, TeamId>,
    annexe_c_map: &Option<HashMap<char, char>>,
    knockout_winners: &HashMap<GameId, TeamId>,
    knockout_losers: &HashMap<GameId, TeamId>,
) -> Option<TeamId> {
    let desc = desc.trim();
    if desc.is_empty() {
        return None;
    }

    // "1X" — winner of group X
    if let Some(letter) = parse_group_position(desc, '1') {
        return group_standings
            .get(&letter)
            .and_then(|gs| gs.order.first().cloned());
    }

    // "2X" — runner-up of group X
    if let Some(letter) = parse_group_position(desc, '2') {
        return group_standings
            .get(&letter)
            .and_then(|gs| gs.order.get(1).cloned());
    }

    // "3ABCDF" or "3X" (direct third)
    if let Some(groups_str) = desc.strip_prefix('3') {
        if groups_str.len() == 1 {
            let letter = groups_str.chars().next().unwrap();
            return qualifying_thirds.get(&letter).cloned();
        }
        if groups_str.len() > 1 && groups_str.chars().all(|c| c.is_ascii_uppercase()) {
            return resolve_best_third_slot(groups_str, annexe_c_map, qualifying_thirds);
        }
    }

    // "Winner MNN" / "Winner SF N"
    if let Some(game_id) = parse_match_ref(desc, "Winner") {
        return knockout_winners.get(&game_id).cloned();
    }

    // "Loser MNN" / "Loser SF N"
    if let Some(game_id) = parse_match_ref(desc, "Loser") {
        return knockout_losers.get(&game_id).cloned();
    }

    None
}

/// Resolve a "best third from groups" slot using Annexe C.
fn resolve_best_third_slot(
    groups_str: &str,
    annexe_c_map: &Option<HashMap<char, char>>,
    qualifying_thirds: &HashMap<char, TeamId>,
) -> Option<TeamId> {
    let annex = annexe_c_map.as_ref()?;

    let winner_group = match groups_str {
        "ABCDF" => 'E',
        "CDFGH" => 'I',
        "CEFHI" => 'A',
        "EHIJK" => 'L',
        "BEFIJ" => 'D',
        "AEHIJ" => 'G',
        "EFGIJ" => 'B',
        "DEIJL" => 'K',
        _ => return None,
    };

    let third_group = annex.get(&winner_group)?;
    qualifying_thirds.get(third_group).cloned()
}

/// Parse "1X" or "2X" where X is a group letter A-L.
/// Returns the group letter if it matches.
fn parse_group_position(desc: &str, pos_char: char) -> Option<char> {
    let mut chars = desc.chars();
    let first = chars.next()?;
    if first != pos_char {
        return None;
    }
    let letter = chars.next()?;
    if !letter.is_ascii_uppercase() || chars.next().is_some() {
        return None;
    }
    if !('A'..='L').contains(&letter) {
        return None;
    }
    Some(letter)
}

/// Parse "Winner M73", "Winner SF 1", "Loser M101", etc. into a game id.
/// We return the game id in a normalized form matching what the tournament uses.
///
/// Recognized patterns:
///   "Winner MNN" → game id "MNN" (e.g. "M73")
///   "Loser MNN"  → game id "MNN"
///   "Winner SF N" → game id "M10N" (SF1=M101, SF2=M102)
///   "Loser SF N"  → game id "M10N"
///
/// Note: the tournament's game ids must follow the "MNN" naming convention.
fn parse_match_ref(desc: &str, prefix: &str) -> Option<GameId> {
    let rest = desc.strip_prefix(prefix)?.trim();

    // "MNN" form
    if let Some(stripped) = rest.strip_prefix('M').or_else(|| rest.strip_prefix('m')) {
        if let Ok(n) = stripped.parse::<u32>() {
            return Some(format!("M{}", n));
        }
    }

    // "SF N" form
    if let Some(sf_rest) = rest.strip_prefix("SF ") {
        if let Ok(n) = sf_rest.trim().parse::<u32>() {
            // SF1 = M101, SF2 = M102
            return Some(format!("M{}", 100 + n));
        }
    }

    // "QF N" form (not in spec but defensive)
    if let Some(qf_rest) = rest.strip_prefix("QF ") {
        if let Ok(n) = qf_rest.trim().parse::<u32>() {
            return Some(format!("M{}", 96 + n));
        }
    }

    None
}

// Re-export best_thirds for use in tests since GroupChildren is needed too
pub use annexe_c_data::ANNEXE_C;
