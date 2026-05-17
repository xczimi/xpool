//! Loud validation of an imported `domain::Tournament`.
//!
//! Catches structural errors that would otherwise surface as confusing
//! runtime behaviour: wrong game count, malformed group stage, dangling
//! references, or missing knockout placeholders.

use anyhow::bail;
use domain::{GroupChildren, Round, Tournament};

/// Expected counts for the FIFA World Cup 26 tournament.
const EXPECTED_GAMES: usize = 104;
const EXPECTED_GROUP_STAGE_GROUPS: usize = 12;
const TEAMS_PER_GROUP: usize = 6; // 6 games per group of 4 teams

/// Validate a tournament loudly. Returns `Err` with a clear message on the
/// first problem found.
pub fn validate(t: &Tournament) -> anyhow::Result<()> {
    // 1. Game count.
    if t.games.len() != EXPECTED_GAMES {
        bail!("expected {EXPECTED_GAMES} games, found {}", t.games.len());
    }

    // 2. Root must resolve.
    if !t.groups.contains_key(&t.root) {
        bail!("root group `{}` is not present in groups", t.root);
    }

    // 3. Group-stage leaf groups: exactly 12, each with 6 games.
    let group_stage_leaves: Vec<_> = t
        .groups
        .values()
        .filter(|g| g.round == Round::GroupStage && matches!(g.children, GroupChildren::Games(_)))
        .collect();
    if group_stage_leaves.len() != EXPECTED_GROUP_STAGE_GROUPS {
        bail!(
            "expected {EXPECTED_GROUP_STAGE_GROUPS} group-stage groups, found {}",
            group_stage_leaves.len()
        );
    }
    for g in &group_stage_leaves {
        if let GroupChildren::Games(ids) = &g.children {
            if ids.len() != TEAMS_PER_GROUP {
                bail!(
                    "group-stage group `{}` has {} games, expected {TEAMS_PER_GROUP}",
                    g.id,
                    ids.len()
                );
            }
        }
    }

    // 4. Every group's children references resolve.
    for g in t.groups.values() {
        match &g.children {
            GroupChildren::Groups(ids) => {
                for id in ids {
                    if !t.groups.contains_key(id) {
                        bail!("group `{}` references missing child group `{id}`", g.id);
                    }
                }
            }
            GroupChildren::Games(ids) => {
                for id in ids {
                    if !t.games.contains_key(id) {
                        bail!("group `{}` references missing game `{id}`", g.id);
                    }
                }
            }
        }
        if let Some(parent) = &g.parent {
            if !t.groups.contains_key(parent) {
                bail!("group `{}` references missing parent `{parent}`", g.id);
            }
        }
    }

    // 5. Every game's group_id resolves; resolved team slots reference teams.
    for game in t.games.values() {
        if !t.groups.contains_key(&game.group_id) {
            bail!(
                "game `{}` references missing group `{}`",
                game.id,
                game.group_id
            );
        }
        for (side, slot) in [("home", &game.home), ("away", &game.away)] {
            if let Some(tid) = &slot.team_id {
                if !t.teams.contains_key(tid) {
                    bail!(
                        "game `{}` {side} slot references missing team `{tid}`",
                        game.id
                    );
                }
            }
        }
    }

    // 6. Knockout games carry placeholder descriptions (non-empty).
    let knockout_games = t.games.values().filter(|g| {
        t.groups
            .get(&g.group_id)
            .is_some_and(|grp| grp.round != Round::GroupStage)
    });
    for game in knockout_games {
        if game.home.description.trim().is_empty() || game.away.description.trim().is_empty() {
            bail!(
                "knockout game `{}` is missing a placeholder description",
                game.id
            );
        }
    }

    Ok(())
}
