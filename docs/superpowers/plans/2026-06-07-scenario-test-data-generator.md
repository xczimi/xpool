# Scenario Test-Data Generator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Generate realistic + whacky full-tournament scenarios (outcome-sets usable as official results or player predictions) as deterministic, repeatable test data, and make the scoreboard re-materialise correctly as of any dev-clock instant from a single seed.

**Architecture:** A new `scenario` module in `crates/xtask` forward-simulates the tournament: a per-game `ScorelinePolicy` (realistic strength-weighted, or fixed whacky archetypes) feeds an engine that reuses `domain::rank_group` and `fwc26::resolve_bracket` so every outcome-set is internally coherent. A new `xtask scenario <id>` subcommand seeds 3 scenarios (~12 players each). The API's `recompute` gains an as-of slice (result-user filtered to matches played by `now`), exposed via a dev-only `devRematerialize` GraphQL mutation that the SPA dev-clock picker calls on every clock change.

**Tech Stack:** Rust (xtask, domain, fwc26, storage, api crates), `rand` (seeded `StdRng`), async-graphql, React + urql (web), Playwright (e2e).

**Spec:** `docs/superpowers/specs/2026-06-07-scenario-test-data-generator-design.md`

---

## File Structure

| File | Responsibility |
| --- | --- |
| `tournaments/fwc26-rankings.json` | Create: flat `{team_id: strength}` map for all 48 teams. Generator-only strength signal; domain `Team` untouched. |
| `crates/xtask/src/scenario/ranking.rs` | Create: load + validate the rankings file. |
| `crates/xtask/src/scenario/policy.rs` | Create: `ScorelinePolicy` trait, `GameContext`, whacky archetypes, `Realistic`, deterministic seed helper. |
| `crates/xtask/src/scenario/engine.rs` | Create: forward-simulate a coherent `Outcome` from a policy. |
| `crates/xtask/src/scenario/scenarios.rs` | Create: the 3 scenario definitions + roster→policy mapping + outcome assembly. |
| `crates/xtask/src/scenario/mod.rs` | Create: `seed_scenario` (seeds players/pool with generated outcomes) + re-exports. |
| `crates/xtask/src/seed.rs` | Modify: expose `pub(crate)` helpers + add `put_player_with_identity`. |
| `crates/xtask/src/lib.rs` | Modify: add `pub mod scenario;`. |
| `crates/xtask/src/main.rs` | Modify: add the `Scenario { id }` subcommand. |
| `crates/xtask/Cargo.toml` | Modify: add `rand = "0.8"`. |
| `crates/api/src/recompute.rs` | Modify: add `slice_result_as_of` + use the sliced result-user for scoring and bracket. |
| `crates/api/src/gql/mutation.rs` | Modify: add the `dev_rematerialize` mutation. |
| `web/src/graphql/queries.ts` | Modify: add `REMATERIALIZE_MUTATION`. |
| `web/src/components/DevClock.tsx` | Modify: call the mutation on apply + reset. |
| `web/e2e/scenario-scoreboard.spec.ts` | Create: seed a scenario, sweep the clock, assert the scoreboard changes. |

---

## Task 1: Rankings file + loader

**Files:**
- Create: `tournaments/fwc26-rankings.json`
- Create: `crates/xtask/src/scenario/ranking.rs`
- Modify: `crates/xtask/src/lib.rs`

- [ ] **Step 1: Create the rankings JSON**

Create `tournaments/fwc26-rankings.json` (every one of the 48 FWC26 team ids, unique strengths so ordering is total):

```json
{
  "ARG": 96, "FRA": 95, "ESP": 94, "ENG": 93, "BRA": 92, "POR": 91,
  "NED": 90, "BEL": 89, "GER": 88, "CRO": 86, "URU": 85, "COL": 84,
  "MAR": 83, "USA": 80, "MEX": 79, "SUI": 78, "JPN": 77, "SEN": 76,
  "IRN": 74, "KOR": 73, "ECU": 72, "AUS": 70, "AUT": 69, "NOR": 68,
  "SCO": 66, "TUR": 65, "UZB": 63, "EGY": 62, "CIV": 61, "PAR": 60,
  "COD": 58, "GHA": 57, "CPV": 55, "PAN": 54, "NZL": 53, "JOR": 52,
  "IRQ": 51, "KSA": 50, "ALG": 49, "TUN": 48, "QAT": 46, "CAN": 45,
  "RSA": 44, "CZE": 43, "SWE": 42, "HAI": 41, "CUW": 40, "BIH": 39
}
```

- [ ] **Step 2: Register the module**

In `crates/xtask/src/lib.rs`, add after `pub mod seed;`:

```rust
pub mod scenario;
```

Create `crates/xtask/src/scenario/mod.rs` with just:

```rust
//! Scenario test-data generator (see
//! `docs/superpowers/specs/2026-06-07-scenario-test-data-generator-design.md`).

pub mod ranking;
```

- [ ] **Step 3: Write the failing test**

Create `crates/xtask/src/scenario/ranking.rs`:

```rust
//! Team-strength input for the generator. A flat `{team_id: strength}` map kept
//! out of the domain `Team` contract — it is a generator-only signal.

use anyhow::{bail, Context};
use domain::Tournament;
use std::collections::HashMap;
use std::path::Path;

/// Per-team strength. Higher = stronger. Drives upset probability and "chalk".
#[derive(Clone, Debug)]
pub struct Ranking {
    strengths: HashMap<String, u32>,
}

impl Ranking {
    /// Load and parse the rankings JSON. Does not validate against a tournament.
    pub fn load(path: &Path) -> anyhow::Result<Ranking> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading rankings file `{}`", path.display()))?;
        let strengths: HashMap<String, u32> = serde_json::from_str(&raw)
            .with_context(|| format!("parsing rankings JSON `{}`", path.display()))?;
        Ok(Ranking { strengths })
    }

    /// Strength for a team; 0 if absent (callers should `validate` first).
    pub fn strength(&self, team: &str) -> u32 {
        self.strengths.get(team).copied().unwrap_or(0)
    }

    /// Every tournament team must have a strength, or this errors loudly.
    pub fn validate(&self, t: &Tournament) -> anyhow::Result<()> {
        let missing: Vec<&String> = t
            .teams
            .keys()
            .filter(|id| !self.strengths.contains_key(*id))
            .collect();
        if !missing.is_empty() {
            bail!("rankings missing strengths for teams: {missing:?}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap as Map;

    fn tournament_with_teams(ids: &[&str]) -> Tournament {
        let teams = ids
            .iter()
            .map(|id| {
                (
                    id.to_string(),
                    domain::Team {
                        id: id.to_string(),
                        name: id.to_string(),
                        short_code: id.to_string(),
                        flag: None,
                        external_id: None,
                    },
                )
            })
            .collect();
        Tournament {
            root: "ROOT".into(),
            groups: Map::new(),
            games: Map::new(),
            teams,
        }
    }

    #[test]
    fn strength_reads_back() {
        let r = Ranking {
            strengths: Map::from([("ARG".to_string(), 96), ("BIH".to_string(), 39)]),
        };
        assert_eq!(r.strength("ARG"), 96);
        assert_eq!(r.strength("BIH"), 39);
        assert_eq!(r.strength("???"), 0);
    }

    #[test]
    fn validate_flags_missing_team() {
        let r = Ranking {
            strengths: Map::from([("ARG".to_string(), 96)]),
        };
        assert!(r.validate(&tournament_with_teams(&["ARG"])).is_ok());
        assert!(r.validate(&tournament_with_teams(&["ARG", "BIH"])).is_err());
    }

    #[test]
    fn real_file_covers_every_team() {
        // The shipped file must cover the real tournament.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tournaments/fwc26.json");
        let t = crate::load_tournament(&path).expect("load tournament");
        let rpath = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tournaments/fwc26-rankings.json");
        let r = Ranking::load(&rpath).expect("load rankings");
        r.validate(&t).expect("every team ranked");
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p xtask scenario::ranking`
Expected: PASS (3 tests). `real_file_covers_every_team` proves the JSON has all 48 teams.

- [ ] **Step 5: Commit**

```bash
git add tournaments/fwc26-rankings.json crates/xtask/src/scenario crates/xtask/src/lib.rs
git commit -m "feat(scenario): team-strength rankings file + validated loader"
```

---

## Task 2: Whacky scoreline policies

**Files:**
- Create: `crates/xtask/src/scenario/policy.rs`
- Modify: `crates/xtask/src/scenario/mod.rs`
- Modify: `crates/xtask/Cargo.toml`

- [ ] **Step 1: Add the `rand` dependency**

In `crates/xtask/Cargo.toml`, under `[dependencies]`, add:

```toml
rand = "0.8"
```

- [ ] **Step 2: Register the module**

In `crates/xtask/src/scenario/mod.rs`, add under `pub mod ranking;`:

```rust
pub mod policy;
```

- [ ] **Step 3: Write the failing test**

Create `crates/xtask/src/scenario/policy.rs`:

```rust
//! Per-game scoreline policies. A policy turns a `GameContext` into a 90-minute
//! score; the engine handles coherence (standings, bracket, advancers).

use domain::Round;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// What a policy sees about one match.
pub struct GameContext {
    pub home: String,
    pub away: String,
    pub home_strength: u32,
    pub away_strength: u32,
    pub round: Round,
}

/// Produce a 90-minute `(home, away)` score for a game.
pub trait ScorelinePolicy {
    fn score(&mut self, ctx: &GameContext) -> (u8, u8);
}

/// Stable 64-bit seed from a scenario + player id (FNV-1a — version-stable,
/// unlike `DefaultHasher`).
pub fn seed_for(scenario_id: &str, player_id: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in scenario_id
        .bytes()
        .chain(b"::".iter().copied())
        .chain(player_id.bytes())
    {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// "Always 1-0 to the home side."
pub struct AlwaysHome;
impl ScorelinePolicy for AlwaysHome {
    fn score(&mut self, _ctx: &GameContext) -> (u8, u8) {
        (1, 0)
    }
}

/// "Always a 1-1 draw."
pub struct AlwaysDraw;
impl ScorelinePolicy for AlwaysDraw {
    fn score(&mut self, _ctx: &GameContext) -> (u8, u8) {
        (1, 1)
    }
}

/// "Chalk": the stronger side wins 1-0; ties go to the home side.
pub struct Chalk;
impl ScorelinePolicy for Chalk {
    fn score(&mut self, ctx: &GameContext) -> (u8, u8) {
        if ctx.home_strength >= ctx.away_strength {
            (1, 0)
        } else {
            (0, 1)
        }
    }
}

/// "Homer": one favourite team always wins big; games without it are 1-1.
pub struct Homer {
    pub fav: String,
}
impl ScorelinePolicy for Homer {
    fn score(&mut self, ctx: &GameContext) -> (u8, u8) {
        if ctx.home == self.fav {
            (3, 0)
        } else if ctx.away == self.fav {
            (0, 3)
        } else {
            (1, 1)
        }
    }
}

/// "Chaos": uniform-random scores 0..=4 each side.
pub struct Chaos {
    rng: StdRng,
}
impl Chaos {
    pub fn new(seed: u64) -> Self {
        Chaos {
            rng: StdRng::seed_from_u64(seed),
        }
    }
}
impl ScorelinePolicy for Chaos {
    fn score(&mut self, _ctx: &GameContext) -> (u8, u8) {
        (self.rng.gen_range(0..=4), self.rng.gen_range(0..=4))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(home: &str, hs: u32, away: &str, as_: u32) -> GameContext {
        GameContext {
            home: home.into(),
            away: away.into(),
            home_strength: hs,
            away_strength: as_,
            round: Round::GroupStage,
        }
    }

    #[test]
    fn always_home_is_one_nil() {
        assert_eq!(AlwaysHome.score(&ctx("A", 1, "B", 9)), (1, 0));
    }

    #[test]
    fn always_draw_is_one_one() {
        assert_eq!(AlwaysDraw.score(&ctx("A", 1, "B", 9)), (1, 1));
    }

    #[test]
    fn chalk_favours_the_stronger_side_and_never_upsets() {
        assert_eq!(Chalk.score(&ctx("A", 9, "B", 1)), (1, 0)); // home stronger
        assert_eq!(Chalk.score(&ctx("A", 1, "B", 9)), (0, 1)); // away stronger
        assert_eq!(Chalk.score(&ctx("A", 5, "B", 5)), (1, 0)); // tie → home
    }

    #[test]
    fn homer_pumps_its_favourite() {
        let mut h = Homer { fav: "BRA".into() };
        assert_eq!(h.score(&ctx("BRA", 5, "X", 5)), (3, 0));
        assert_eq!(h.score(&ctx("X", 5, "BRA", 5)), (0, 3));
        assert_eq!(h.score(&ctx("X", 5, "Y", 5)), (1, 1));
    }

    #[test]
    fn chaos_is_deterministic_for_a_seed() {
        let mut a = Chaos::new(seed_for("s", "p"));
        let mut b = Chaos::new(seed_for("s", "p"));
        for _ in 0..20 {
            assert_eq!(a.score(&ctx("X", 5, "Y", 5)), b.score(&ctx("X", 5, "Y", 5)));
        }
    }

    #[test]
    fn seed_for_is_stable_and_distinct() {
        assert_eq!(seed_for("chalk", "demo-ada"), seed_for("chalk", "demo-ada"));
        assert_ne!(seed_for("chalk", "demo-ada"), seed_for("chalk", "demo-alan"));
        assert_ne!(seed_for("chalk", "demo-ada"), seed_for("chaos", "demo-ada"));
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p xtask scenario::policy`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/xtask/Cargo.toml crates/xtask/src/scenario/policy.rs crates/xtask/src/scenario/mod.rs
git commit -m "feat(scenario): scoreline policy trait + whacky archetypes + seed helper"
```

---

## Task 3: Realistic strength-weighted policy

**Files:**
- Modify: `crates/xtask/src/scenario/policy.rs`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `crates/xtask/src/scenario/policy.rs`:

```rust
    #[test]
    fn realistic_with_zero_upset_always_favours_the_stronger_side() {
        let mut p = Realistic::new(seed_for("balanced", "demo-ada"), 0.0);
        for _ in 0..50 {
            let (h, a) = p.score(&ctx("STRONG", 90, "WEAK", 40));
            assert!(h > a, "stronger home should win: {h}-{a}");
            let (h, a) = p.score(&ctx("WEAK", 40, "STRONG", 90));
            assert!(a > h, "stronger away should win: {h}-{a}");
        }
    }

    #[test]
    fn realistic_is_reproducible_for_a_seed() {
        let mut a = Realistic::new(seed_for("balanced", "demo-ada"), 0.3);
        let mut b = Realistic::new(seed_for("balanced", "demo-ada"), 0.3);
        for _ in 0..50 {
            assert_eq!(
                a.score(&ctx("X", 70, "Y", 55)),
                b.score(&ctx("X", 70, "Y", 55))
            );
        }
    }

    #[test]
    fn realistic_high_upset_sometimes_flips_the_favourite() {
        let mut p = Realistic::new(seed_for("balanced", "underdog"), 1.0);
        // upset_prob = 1.0 → the weaker side always wins.
        for _ in 0..20 {
            let (h, a) = p.score(&ctx("STRONG", 90, "WEAK", 40));
            assert!(a > h, "forced upset: {h}-{a}");
        }
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p xtask scenario::policy::tests::realistic_with_zero_upset`
Expected: FAIL — `cannot find type Realistic in this scope`.

- [ ] **Step 3: Write the implementation**

Add to `crates/xtask/src/scenario/policy.rs` (above the `#[cfg(test)]` module):

```rust
/// "Realistic": the stronger side usually wins, with a tunable `upset_prob`
/// (0.0 = strict chalk-with-margins, 1.0 = the weaker side always wins). Never
/// produces a draw — the favourite (or the upset winner) scores one more.
pub struct Realistic {
    rng: StdRng,
    upset_prob: f64,
}

impl Realistic {
    pub fn new(seed: u64, upset_prob: f64) -> Self {
        Realistic {
            rng: StdRng::seed_from_u64(seed),
            upset_prob: upset_prob.clamp(0.0, 1.0),
        }
    }
}

impl ScorelinePolicy for Realistic {
    fn score(&mut self, ctx: &GameContext) -> (u8, u8) {
        let home_is_strong = ctx.home_strength >= ctx.away_strength;
        let upset = self.rng.gen::<f64>() < self.upset_prob;
        // Winner = the strong side unless this is an upset.
        let home_wins = home_is_strong ^ upset;

        let winner_goals: u8 = self.rng.gen_range(1..=3);
        let loser_goals: u8 = self.rng.gen_range(0..winner_goals);

        if home_wins {
            (winner_goals, loser_goals)
        } else {
            (loser_goals, winner_goals)
        }
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p xtask scenario::policy`
Expected: PASS (9 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/xtask/src/scenario/policy.rs
git commit -m "feat(scenario): realistic strength-weighted policy with tunable upset prob"
```

---

## Task 4: Engine — group-stage generation + standings

**Files:**
- Create: `crates/xtask/src/scenario/engine.rs`
- Modify: `crates/xtask/src/scenario/mod.rs`

- [ ] **Step 1: Register the module**

In `crates/xtask/src/scenario/mod.rs`, add under `pub mod policy;`:

```rust
pub mod engine;
```

- [ ] **Step 2: Write the failing test**

Create `crates/xtask/src/scenario/engine.rs`:

```rust
//! Forward-simulate a coherent outcome-set from a `ScorelinePolicy`. Every
//! knockout pairing is derived from the predictor's own earlier results via the
//! same `fwc26::resolve_bracket` / `domain::rank_group` the live app uses.

use crate::scenario::policy::{GameContext, ScorelinePolicy};
use crate::scenario::ranking::Ranking;
use domain::{
    GroupChildren, MatchPrediction, Player, Round, SingleGame, StandingsPrediction, TeamId,
    Tournament,
};
use std::collections::HashMap;

/// Rounds in the only valid simulation order.
const ROUND_ORDER: [Round; 7] = [
    Round::GroupStage,
    Round::R32,
    Round::R16,
    Round::QF,
    Round::SF,
    Round::ThirdPlace,
    Round::Final,
];

/// A complete generated outcome-set.
pub struct Outcome {
    pub match_predictions: Vec<MatchPrediction>,
    pub standings_predictions: Vec<StandingsPrediction>,
    /// The concrete teams the engine used for each game — for the coherence
    /// round-trip test and debugging.
    pub resolved_teams: HashMap<String, (TeamId, TeamId)>,
}

/// A throwaway result-user-shaped player carrying the predictions so far, so
/// `resolve_bracket` can resolve the next round.
fn interim_player(mps: &[MatchPrediction], sps: &[StandingsPrediction]) -> Player {
    Player {
        id: "__gen".into(),
        person_id: String::new(),
        nick: String::new(),
        full_name: String::new(),
        referrer: None,
        is_result_user: true,
        version: 0,
        match_predictions: mps.to_vec(),
        standings_predictions: sps.to_vec(),
    }
}

/// All `SingleGame`s belonging to a leaf group (`GroupChildren::Games`).
fn group_games<'a>(t: &'a Tournament, group_id: &str) -> Vec<&'a SingleGame> {
    match t.groups.get(group_id).map(|g| &g.children) {
        Some(GroupChildren::Games(ids)) => ids.iter().filter_map(|id| t.games.get(id)).collect(),
        _ => Vec::new(),
    }
}

/// A group's teams sorted by strength descending (stable) — a total draw_order
/// so ranking and bracket resolution are never ambiguous.
fn draw_order_by_strength(team_ids: &[TeamId], ranking: &Ranking) -> Vec<TeamId> {
    let mut ids = team_ids.to_vec();
    ids.sort_by(|a, b| ranking.strength(b).cmp(&ranking.strength(a)));
    ids
}

/// Forward-simulate the whole tournament under `policy`.
pub fn generate(
    t: &Tournament,
    ranking: &Ranking,
    policy: &mut dyn ScorelinePolicy,
) -> Outcome {
    let mut mps: Vec<MatchPrediction> = Vec::new();
    let mut sps: Vec<StandingsPrediction> = Vec::new();
    let mut resolved_teams: HashMap<String, (TeamId, TeamId)> = HashMap::new();

    for round in ROUND_ORDER {
        // Games in this round, ordered deterministically (kickoff, then id).
        let mut games: Vec<&SingleGame> = t
            .games
            .values()
            .filter(|g| t.groups.get(&g.group_id).map(|gr| gr.round) == Some(round))
            .collect();
        games.sort_by(|a, b| a.kickoff.cmp(&b.kickoff).then_with(|| a.id.cmp(&b.id)));

        // Resolve knockout teams for this round from results so far.
        let resolved = if round == Round::GroupStage {
            HashMap::new()
        } else {
            let interim = interim_player(&mps, &sps);
            fwc26::resolve_bracket(t, &interim)
        };

        for game in &games {
            let (home, away) = if round == Round::GroupStage {
                (
                    game.home.team_id.clone().expect("group-stage home team"),
                    game.away.team_id.clone().expect("group-stage away team"),
                )
            } else {
                let (h, a) = resolved.get(&game.id).cloned().unwrap_or((None, None));
                (
                    h.expect("knockout home resolved by now"),
                    a.expect("knockout away resolved by now"),
                )
            };

            let ctx = GameContext {
                home: home.clone(),
                away: away.clone(),
                home_strength: ranking.strength(&home),
                away_strength: ranking.strength(&away),
                round,
            };
            let (hs, as_) = policy.score(&ctx);
            mps.push(MatchPrediction {
                game_id: game.id.clone(),
                home_score: hs,
                away_score: as_,
                locked: false,
            });
            resolved_teams.insert(game.id.clone(), (home.clone(), away.clone()));

            // Knockout: record the advancer in the one-match group's standings
            // so `resolve_bracket` resolves draws the same way next round.
            if round != Round::GroupStage {
                let (adv, elim) = advancer(hs, as_, &home, &away, ranking);
                sps.push(StandingsPrediction {
                    group_id: game.group_id.clone(),
                    ordering: vec![adv.clone(), elim.clone()],
                    draw_order: vec![adv, elim],
                    locked: false,
                });
            }
        }

        // Group-stage standings: rank each leaf group from the scores just set.
        if round == Round::GroupStage {
            for (gid, group) in &t.groups {
                if group.round != Round::GroupStage || !group.carries_standings {
                    continue;
                }
                let games_g = group_games(t, gid);
                if games_g.is_empty() {
                    continue;
                }
                let team_ids: Vec<TeamId> = games_g
                    .iter()
                    .flat_map(|g| {
                        [g.home.team_id.clone(), g.away.team_id.clone()]
                            .into_iter()
                            .flatten()
                    })
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect();
                let draw_order = draw_order_by_strength(&team_ids, ranking);
                let pred_refs: Vec<&MatchPrediction> = games_g
                    .iter()
                    .filter_map(|g| mps.iter().find(|p| p.game_id == g.id))
                    .collect();
                let ordering =
                    domain::rank_group(group, &games_g, &pred_refs, &draw_order);
                sps.push(StandingsPrediction {
                    group_id: gid.clone(),
                    ordering,
                    draw_order,
                    locked: false,
                });
            }
        }
    }

    Outcome {
        match_predictions: mps,
        standings_predictions: sps,
        resolved_teams,
    }
}

/// Decide the knockout advancer: higher score wins; a 90-minute draw goes to the
/// higher-strength side (home on a tie) — matching `fwc26`'s draw fallback.
fn advancer(
    hs: u8,
    as_: u8,
    home: &str,
    away: &str,
    ranking: &Ranking,
) -> (TeamId, TeamId) {
    use std::cmp::Ordering::*;
    match hs.cmp(&as_) {
        Greater => (home.to_string(), away.to_string()),
        Less => (away.to_string(), home.to_string()),
        Equal => {
            if ranking.strength(away) > ranking.strength(home) {
                (away.to_string(), home.to_string())
            } else {
                (home.to_string(), away.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::policy::AlwaysHome;
    use chrono::{TimeZone, Utc};
    use domain::{GroupGame, LockMode, Team, TeamSlot};
    use std::collections::HashMap as Map;

    // A minimal 1-group, 1-game tournament (no knockout) for group-stage tests.
    fn one_group_tournament() -> Tournament {
        let mk_team = |id: &str| Team {
            id: id.into(),
            name: id.into(),
            short_code: id.into(),
            flag: None,
            external_id: None,
        };
        let game = SingleGame {
            id: "G1".into(),
            kickoff: Utc.with_ymd_and_hms(2026, 6, 11, 19, 0, 0).unwrap(),
            venue: None,
            group_id: "A".into(),
            home: TeamSlot { team_id: Some("HOME".into()), description: "A1".into() },
            away: TeamSlot { team_id: Some("AWAY".into()), description: "A2".into() },
        };
        let group = GroupGame {
            id: "A".into(),
            name: "Group A".into(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec!["G1".into()]),
        };
        Tournament {
            root: "A".into(),
            groups: Map::from([("A".to_string(), group)]),
            games: Map::from([("G1".to_string(), game)]),
            teams: Map::from([
                ("HOME".to_string(), mk_team("HOME")),
                ("AWAY".to_string(), mk_team("AWAY")),
            ]),
        }
    }

    fn ranking() -> Ranking {
        // HOME stronger than AWAY. `Ranking` derives transparent `Deserialize`
        // (added in Step 3), so it builds straight from a JSON object without a
        // public constructor and the test does not depend on real team ids.
        serde_json::from_value(serde_json::json!({ "HOME": 80, "AWAY": 40 })).unwrap()
    }

    #[test]
    fn group_stage_produces_a_match_and_a_standings_row() {
        let t = one_group_tournament();
        let r = ranking();
        let out = generate(&t, &r, &mut AlwaysHome);

        // One match prediction, 1-0 to HOME.
        assert_eq!(out.match_predictions.len(), 1);
        let mp = &out.match_predictions[0];
        assert_eq!((mp.home_score, mp.away_score), (1, 0));
        assert!(!mp.locked);

        // One standings row, HOME ranked first (it won).
        let sp = out
            .standings_predictions
            .iter()
            .find(|s| s.group_id == "A")
            .expect("group A standings");
        assert_eq!(sp.ordering.first().map(String::as_str), Some("HOME"));
    }
}
```

Note: `Ranking` needs to deserialize from JSON in the shim. Confirm `Ranking` derives `serde::Deserialize` — if not, add it in the next step.

- [ ] **Step 3: Make `Ranking` constructible from JSON in tests**

In `crates/xtask/src/scenario/ranking.rs`, change the struct to derive `Deserialize` over a transparent map so `serde_json::from_str` yields a `Ranking` directly:

```rust
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(transparent)]
pub struct Ranking {
    strengths: HashMap<String, u32>,
}
```

Keep `load` as-is but simplify its body to reuse the derive:

```rust
    pub fn load(path: &Path) -> anyhow::Result<Ranking> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading rankings file `{}`", path.display()))?;
        serde_json::from_str(&raw)
            .with_context(|| format!("parsing rankings JSON `{}`", path.display()))
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p xtask scenario::engine`
Expected: PASS (1 test). Also re-run `cargo test -p xtask scenario::ranking` — still PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/xtask/src/scenario/engine.rs crates/xtask/src/scenario/ranking.rs crates/xtask/src/scenario/mod.rs
git commit -m "feat(scenario): engine group-stage generation + standings"
```

---

## Task 5: Engine — knockout progression (coherence on real tournament)

**Files:**
- Modify: `crates/xtask/src/scenario/engine.rs`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `crates/xtask/src/scenario/engine.rs`:

```rust
    use crate::scenario::policy::{Chalk, AlwaysDraw};

    fn real_tournament() -> Tournament {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tournaments/fwc26.json");
        crate::load_tournament(&path).expect("load fwc26")
    }

    fn real_ranking() -> Ranking {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tournaments/fwc26-rankings.json");
        Ranking::load(&path).expect("load rankings")
    }

    #[test]
    fn full_tournament_resolves_every_game_including_the_final() {
        let t = real_tournament();
        let r = real_ranking();
        let out = generate(&t, &r, &mut Chalk);

        // Every game in the tournament gets a prediction and resolved teams.
        assert_eq!(out.match_predictions.len(), t.games.len());
        assert_eq!(out.resolved_teams.len(), t.games.len());

        // No knockout game left with a placeholder (all teams concrete).
        for (gid, (home, away)) in &out.resolved_teams {
            assert!(!home.is_empty(), "game {gid} home unresolved");
            assert!(!away.is_empty(), "game {gid} away unresolved");
        }
    }

    #[test]
    fn coherence_round_trip_matches_resolve_bracket() {
        let t = real_tournament();
        let r = real_ranking();
        // AlwaysDraw stresses the advancer path (every knockout is a 1-1 draw).
        let out = generate(&t, &r, &mut AlwaysDraw);

        // Assign the outcome as the result-user and re-resolve the bracket.
        let result = interim_player(&out.match_predictions, &out.standings_predictions);
        let bracket = fwc26::resolve_bracket(&t, &result);

        // For every knockout game, resolve_bracket must reproduce exactly the
        // teams the engine used.
        for (gid, game) in &t.games {
            let round = t.groups.get(&game.group_id).map(|g| g.round);
            if round == Some(Round::GroupStage) {
                continue;
            }
            let (eh, ea) = out.resolved_teams.get(gid).expect("engine teams");
            let (rh, ra) = bracket.get(gid).cloned().unwrap_or((None, None));
            assert_eq!(rh.as_ref(), Some(eh), "home mismatch on {gid}");
            assert_eq!(ra.as_ref(), Some(ea), "away mismatch on {gid}");
        }
    }
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p xtask scenario::engine`
Expected: PASS. If `coherence_round_trip_matches_resolve_bracket` fails on a draw-advancer mismatch, the engine's `advancer` and `fwc26::determine_winner_loser` disagree — verify the one-match group's `ordering[0]` is the advancer (it is, per `crates/fwc26/src/lib.rs:582`). The group-stage `generate` already runs before knockout rounds, so standings exist when `resolve_bracket` is first called.

- [ ] **Step 3: Fix only if a test fails**

If `full_tournament_resolves_every_game...` panics with "knockout home resolved by now", a round's games reference an earlier round not yet processed — confirm `ROUND_ORDER` covers every distinct `round` value present in `t.groups`. No code change is expected if Task 4 was implemented as written.

- [ ] **Step 4: Run the full crate test suite**

Run: `cargo test -p xtask`
Expected: PASS (all scenario + existing seed/import tests).

- [ ] **Step 5: Commit**

```bash
git add crates/xtask/src/scenario/engine.rs
git commit -m "test(scenario): full-tournament coherence round-trip vs resolve_bracket"
```

---

## Task 6: Scenario definitions + outcome assembly

**Files:**
- Create: `crates/xtask/src/scenario/scenarios.rs`
- Modify: `crates/xtask/src/scenario/mod.rs`

- [ ] **Step 1: Register the module**

In `crates/xtask/src/scenario/mod.rs`, add under `pub mod engine;`:

```rust
pub mod scenarios;
```

- [ ] **Step 2: Write the failing test**

Create `crates/xtask/src/scenario/scenarios.rs`:

```rust
//! The fixed scenario roster: 3 scenarios, each a result-user policy plus the
//! shared body of predictors (6 realistic demo players + 5 whacky archetypes).

use crate::scenario::engine::{generate, Outcome};
use crate::scenario::policy::{
    seed_for, AlwaysDraw, AlwaysHome, Chalk, Chaos, Homer, Realistic, ScorelinePolicy,
};
use crate::scenario::ranking::Ranking;
use domain::Tournament;

/// Which scoreline policy a player (or the result-user) plays.
#[derive(Clone, Debug)]
pub enum PolicyKind {
    Realistic { upset_prob: f64 },
    AlwaysHome,
    AlwaysDraw,
    Chaos,
    Homer { fav: String },
    Chalk,
}

/// A predictor in a scenario.
#[derive(Clone, Debug)]
pub struct PlayerSpec {
    pub id: String,
    pub nick: String,
    pub full_name: String,
    pub policy: PolicyKind,
    /// True for the 6 demo players seeded by `seed()`; false for whacky players
    /// the scenario seeder must create.
    pub preexisting: bool,
}

/// One full scenario.
#[derive(Clone, Debug)]
pub struct Scenario {
    pub id: String,
    pub result_policy: PolicyKind,
    pub players: Vec<PlayerSpec>,
}

/// The 6 demo players (already seeded) + 5 whacky archetypes (created fresh).
fn roster() -> Vec<PlayerSpec> {
    let demo = [
        ("demo-ada", "ada", "Ada Lovelace"),
        ("demo-alan", "alan", "Alan Turing"),
        ("demo-grace", "grace", "Grace Hopper"),
        ("demo-linus", "linus", "Linus Torvalds"),
        ("demo-margaret", "margaret", "Margaret Hamilton"),
        ("demo-dennis", "dennis", "Dennis Ritchie"),
    ];
    let mut players: Vec<PlayerSpec> = demo
        .iter()
        .map(|(id, nick, name)| PlayerSpec {
            id: (*id).into(),
            nick: (*nick).into(),
            full_name: (*name).into(),
            policy: PolicyKind::Realistic { upset_prob: 0.25 },
            preexisting: true,
        })
        .collect();

    let whacky = [
        ("whacky-onenil", "onenil", "Mr. One-Nil", PolicyKind::AlwaysHome),
        ("whacky-draw", "stalemate", "Sir Stalemate", PolicyKind::AlwaysDraw),
        ("whacky-chaos", "chaos", "Captain Chaos", PolicyKind::Chaos),
        ("whacky-homer", "homer", "The Homer", PolicyKind::Homer { fav: "BRA".into() }),
        ("whacky-chalk", "formguide", "Form Guide", PolicyKind::Chalk),
    ];
    players.extend(whacky.into_iter().map(|(id, nick, name, policy)| PlayerSpec {
        id: id.into(),
        nick: nick.into(),
        full_name: name.into(),
        policy,
        preexisting: false,
    }));
    players
}

/// All scenarios, keyed by id. The result-user's policy is what makes each
/// scenario's official results distinct.
pub fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            id: "chalk".into(),
            result_policy: PolicyKind::Chalk,
            players: roster(),
        },
        Scenario {
            id: "balanced".into(),
            result_policy: PolicyKind::Realistic { upset_prob: 0.25 },
            players: roster(),
        },
        Scenario {
            id: "chaos".into(),
            result_policy: PolicyKind::Chaos,
            players: roster(),
        },
    ]
}

/// Build a policy for a player in a scenario, seeded deterministically.
pub fn build_policy(kind: &PolicyKind, scenario_id: &str, player_id: &str) -> Box<dyn ScorelinePolicy> {
    let seed = seed_for(scenario_id, player_id);
    match kind {
        PolicyKind::Realistic { upset_prob } => Box::new(Realistic::new(seed, *upset_prob)),
        PolicyKind::AlwaysHome => Box::new(AlwaysHome),
        PolicyKind::AlwaysDraw => Box::new(AlwaysDraw),
        PolicyKind::Chaos => Box::new(Chaos::new(seed)),
        PolicyKind::Homer { fav } => Box::new(Homer { fav: fav.clone() }),
        PolicyKind::Chalk => Box::new(Chalk),
    }
}

/// Generate the result-user outcome for a scenario.
pub fn result_outcome(t: &Tournament, ranking: &Ranking, scenario: &Scenario) -> Outcome {
    let mut policy = build_policy(&scenario.result_policy, &scenario.id, "result-user");
    generate(t, ranking, policy.as_mut())
}

/// Generate one player's outcome for a scenario.
pub fn player_outcome(
    t: &Tournament,
    ranking: &Ranking,
    scenario: &Scenario,
    player: &PlayerSpec,
) -> Outcome {
    let mut policy = build_policy(&player.policy, &scenario.id, &player.id);
    generate(t, ranking, policy.as_mut())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_tournament() -> Tournament {
        crate::load_tournament(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tournaments/fwc26.json"),
        )
        .unwrap()
    }
    fn real_ranking() -> Ranking {
        Ranking::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tournaments/fwc26-rankings.json"),
        )
        .unwrap()
    }

    #[test]
    fn three_scenarios_with_twelve_predictors_each() {
        let s = scenarios();
        assert_eq!(s.len(), 3);
        // 11 players + 1 result-user = 12 outcome-producing entities.
        assert_eq!(s[0].players.len(), 11);
        assert!(s.iter().any(|x| x.id == "balanced"));
    }

    #[test]
    fn onenil_player_predicts_every_group_game_one_nil() {
        let t = real_tournament();
        let r = real_ranking();
        let scenario = scenarios().into_iter().find(|s| s.id == "balanced").unwrap();
        let onenil = scenario
            .players
            .iter()
            .find(|p| p.id == "whacky-onenil")
            .unwrap();
        let out = player_outcome(&t, &r, &scenario, onenil);
        // Every group-stage game is 1-0 (knockout draws get an advancer but the
        // 90-min score AlwaysHome emits is still 1-0).
        assert!(out.match_predictions.iter().all(|mp| (mp.home_score, mp.away_score) == (1, 0)));
    }

    #[test]
    fn result_outcome_is_reproducible() {
        let t = real_tournament();
        let r = real_ranking();
        let scenario = scenarios().into_iter().find(|s| s.id == "chaos").unwrap();
        let a = result_outcome(&t, &r, &scenario);
        let b = result_outcome(&t, &r, &scenario);
        assert_eq!(a.match_predictions, b.match_predictions);
    }
}
```

- [ ] **Step 3: Run the tests to verify they pass**

Run: `cargo test -p xtask scenario::scenarios`
Expected: PASS (3 tests). `MatchPrediction` derives `PartialEq` (confirmed in `crates/domain/src/model.rs:97`), so the reproducibility `assert_eq!` compiles.

- [ ] **Step 4: Commit**

```bash
git add crates/xtask/src/scenario/scenarios.rs crates/xtask/src/scenario/mod.rs
git commit -m "feat(scenario): 3 scenario definitions + deterministic outcome assembly"
```

---

## Task 7: Seed helpers + `seed_scenario`

**Files:**
- Modify: `crates/xtask/src/seed.rs`
- Modify: `crates/xtask/src/scenario/mod.rs`

- [ ] **Step 1: Expose seed helpers**

In `crates/xtask/src/seed.rs`, change the visibility of the two helpers so the scenario module can reuse them. Change:

```rust
fn fresh_player(id: &str, person_id: &str, nick: &str, full_name: &str, is_result: bool) -> Player {
```
to:
```rust
pub(crate) fn fresh_player(id: &str, person_id: &str, nick: &str, full_name: &str, is_result: bool) -> Player {
```

And change:
```rust
async fn put_player_idempotent(repo: &dyn Repository, mut player: Player) -> anyhow::Result<()> {
```
to:
```rust
pub(crate) async fn put_player_idempotent(repo: &dyn Repository, mut player: Player) -> anyhow::Result<()> {
```

- [ ] **Step 2: Write the failing test**

Add to `crates/xtask/src/scenario/mod.rs` (replace its current contents, keeping the `pub mod` lines):

```rust
//! Scenario test-data generator (see
//! `docs/superpowers/specs/2026-06-07-scenario-test-data-generator-design.md`).

pub mod engine;
pub mod policy;
pub mod ranking;
pub mod scenarios;

use crate::scenario::ranking::Ranking;
use crate::scenario::scenarios::{player_outcome, result_outcome, scenarios, Scenario};
use crate::seed::{fresh_player, put_player_idempotent, seed, RESULT_USER_ID};
use domain::{Identity, Person, Player, Pool};
use std::path::Path;
use storage::Repository;

/// Default rankings path (relative to the workspace root).
pub const DEFAULT_RANKINGS_PATH: &str = "tournaments/fwc26-rankings.json";

/// Overwrite an already-seeded player's predictions with a generated outcome,
/// preserving the stored optimistic-concurrency `version`.
async fn apply_outcome(
    repo: &dyn Repository,
    player_id: &str,
    mps: Vec<domain::MatchPrediction>,
    sps: Vec<domain::StandingsPrediction>,
) -> anyhow::Result<()> {
    let existing = repo
        .get_player(player_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("player {player_id} not seeded; run base seed first"))?;
    let updated = Player {
        match_predictions: mps,
        standings_predictions: sps,
        ..existing
    };
    put_player_idempotent(repo, updated).await
}

/// Create a fresh whacky player with a Person + dev-login Identity, carrying its
/// generated outcome. Mirrors the per-player wiring in `seed_with_email`.
async fn create_player(
    repo: &dyn Repository,
    player_id: &str,
    nick: &str,
    full_name: &str,
    mps: Vec<domain::MatchPrediction>,
    sps: Vec<domain::StandingsPrediction>,
) -> anyhow::Result<()> {
    let person_id = format!("person-{nick}");
    let identity_id = format!("identity-{nick}");
    let dev_email = format!("{player_id}@dev.invalid");

    repo.put_identity(&Identity {
        id: identity_id.clone(),
        provider: "email".into(),
        provider_id: dev_email.clone(),
        person_id: person_id.clone(),
        verified_email: Some(dev_email),
    })
    .await?;
    repo.put_person(&Person {
        id: person_id.clone(),
        identity_ids: vec![identity_id],
    })
    .await?;

    let mut player = fresh_player(player_id, &person_id, nick, full_name, false);
    player.match_predictions = mps;
    player.standings_predictions = sps;
    put_player_idempotent(repo, player).await
}

/// Seed a full scenario into the repository: base demo data, then overwrite the
/// result-user + demo players with generated outcomes, create the whacky
/// players, and add everyone to the demo pool. Idempotent.
pub async fn seed_scenario(
    repo: &dyn Repository,
    scenario_id: &str,
    rankings_path: &Path,
) -> anyhow::Result<()> {
    let tournament = repo
        .get_tournament()
        .await?
        .ok_or_else(|| anyhow::anyhow!("no tournament loaded; run `xtask import` first"))?;
    let ranking = Ranking::load(rankings_path)?;
    ranking.validate(&tournament)?;

    let scenario: Scenario = scenarios()
        .into_iter()
        .find(|s| s.id == scenario_id)
        .ok_or_else(|| {
            let ids: Vec<String> = scenarios().into_iter().map(|s| s.id).collect();
            anyhow::anyhow!("unknown scenario `{scenario_id}`; valid: {ids:?}")
        })?;

    // Base identities/persons/players/pool (idempotent).
    seed(repo).await?;

    // Result-user gets the official outcome.
    let r_out = result_outcome(&tournament, &ranking, &scenario);
    apply_outcome(
        repo,
        RESULT_USER_ID,
        r_out.match_predictions,
        r_out.standings_predictions,
    )
    .await?;

    // Each predictor gets its own outcome.
    let mut whacky_ids: Vec<String> = Vec::new();
    for player in &scenario.players {
        let out = player_outcome(&tournament, &ranking, &scenario, player);
        if player.preexisting {
            apply_outcome(repo, &player.id, out.match_predictions, out.standings_predictions)
                .await?;
        } else {
            create_player(
                repo,
                &player.id,
                &player.nick,
                &player.full_name,
                out.match_predictions,
                out.standings_predictions,
            )
            .await?;
            whacky_ids.push(player.id.clone());
        }
    }

    // Add the whacky players to the demo pool so the scoreboard shows everyone.
    if let Some(mut pool) = repo.list_pools().await?.into_iter().find(|p| p.id == "pool-demo") {
        for id in whacky_ids {
            if !pool.members.contains(&id) {
                pool.members.push(id);
            }
        }
        repo.put_pool(&pool).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage::InMemoryRepository;

    async fn seeded_repo(scenario: &str) -> InMemoryRepository {
        let repo = InMemoryRepository::new();
        let t = crate::load_tournament(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tournaments/fwc26.json"),
        )
        .unwrap();
        repo.put_tournament(&t).await.unwrap();
        let rpath = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tournaments/fwc26-rankings.json");
        seed_scenario(&repo, scenario, &rpath).await.unwrap();
        repo
    }

    #[tokio::test]
    async fn result_user_and_players_get_predictions() {
        let repo = seeded_repo("balanced").await;

        let result = repo.get_player(RESULT_USER_ID).await.unwrap().unwrap();
        assert!(!result.match_predictions.is_empty(), "result-user has results");
        assert!(result.is_result_user);

        let ada = repo.get_player("demo-ada").await.unwrap().unwrap();
        assert!(!ada.match_predictions.is_empty(), "demo player has predictions");

        let onenil = repo.get_player("whacky-onenil").await.unwrap().unwrap();
        assert!(onenil
            .match_predictions
            .iter()
            .all(|mp| (mp.home_score, mp.away_score) == (1, 0)));
    }

    #[tokio::test]
    async fn whacky_players_are_dev_loginable_and_in_the_pool() {
        let repo = seeded_repo("chalk").await;

        // Identity resolvable by dev-login email.
        let id = repo
            .get_identity("email", "whacky-chaos@dev.invalid")
            .await
            .unwrap()
            .expect("whacky identity exists");
        let player = repo
            .get_player_by_person(&id.person_id)
            .await
            .unwrap()
            .expect("resolves to a player");
        assert_eq!(player.id, "whacky-chaos");

        let pool = repo.list_pools().await.unwrap().into_iter().find(|p| p.id == "pool-demo").unwrap();
        assert!(pool.members.contains(&"whacky-chaos".to_string()));
        assert_eq!(pool.members.len(), 11); // 6 demo + 5 whacky
    }

    #[tokio::test]
    async fn unknown_scenario_errors() {
        let repo = InMemoryRepository::new();
        let t = crate::load_tournament(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tournaments/fwc26.json"),
        )
        .unwrap();
        repo.put_tournament(&t).await.unwrap();
        let rpath = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tournaments/fwc26-rankings.json");
        let err = seed_scenario(&repo, "nope", &rpath).await.unwrap_err();
        assert!(err.to_string().contains("unknown scenario"));
    }
}
```

- [ ] **Step 3: Confirm `RESULT_USER_ID` is exported**

`crates/xtask/src/seed.rs:11` already declares `pub const RESULT_USER_ID`. No change needed.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p xtask scenario::tests`
Expected: PASS (3 tests). These use `InMemoryRepository`, so no DynamoDB is required.

- [ ] **Step 5: Commit**

```bash
git add crates/xtask/src/seed.rs crates/xtask/src/scenario/mod.rs
git commit -m "feat(scenario): seed_scenario wires generated outcomes into players + pool"
```

---

## Task 8: `xtask scenario <id>` subcommand

**Files:**
- Modify: `crates/xtask/src/main.rs`

- [ ] **Step 1: Add the subcommand variant**

In `crates/xtask/src/main.rs`, add to the `Command` enum (after `DropTable`):

```rust
    /// Seed a generated scenario (full results + ~12 players' predictions).
    Scenario {
        /// Scenario id: `chalk`, `balanced`, or `chaos`.
        id: String,
    },
```

- [ ] **Step 2: Handle it in `main`**

In `crates/xtask/src/main.rs`, add a match arm in the `match cli.command` block (after the `Command::DropTable` arm):

```rust
        Command::Scenario { id } => {
            repo.ensure_table().await?;
            let rankings = std::path::PathBuf::from(xtask::scenario::DEFAULT_RANKINGS_PATH);
            xtask::scenario::seed_scenario(&repo, &id, &rankings).await?;
            println!(
                "seeded scenario `{id}`: official results + 6 demo + 5 whacky players. \
                 Move the dev clock and call the `devRematerialize` mutation to build the \
                 scoreboard as-of that time."
            );
        }
```

- [ ] **Step 3: Verify it builds and the CLI lists the subcommand**

Run: `cargo run -p xtask -- scenario --help`
Expected: usage text showing the `id` argument. (No DynamoDB needed for `--help`.)

- [ ] **Step 4: Manual smoke (optional, needs Docker)**

If DynamoDB Local is running (`docker compose up -d`, `export DYNAMO_ENDPOINT=http://localhost:8000`):

Run:
```bash
cargo run -p xtask -- import tournaments/fwc26.json
cargo run -p xtask -- scenario balanced
```
Expected: "seeded scenario `balanced`: ..." with no error.

- [ ] **Step 5: Commit**

```bash
git add crates/xtask/src/main.rs
git commit -m "feat(scenario): xtask scenario <id> subcommand"
```

---

## Task 9: As-of slice in `recompute`

**Files:**
- Modify: `crates/api/src/recompute.rs`

- [ ] **Step 1: Write the failing test**

Add a `#[cfg(test)]` module at the bottom of `crates/api/src/recompute.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use domain::{
        GroupChildren, GroupGame, LockMode, MatchPrediction, Player, Round, SingleGame,
        StandingsPrediction, Team, TeamSlot, Tournament,
    };
    use std::collections::HashMap;

    fn at(y: i32, mo: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, 0, 0).unwrap()
    }

    // Two group-stage games in group A, kickoffs a day apart.
    fn fixture() -> (Tournament, Player) {
        let team = |id: &str| Team {
            id: id.into(),
            name: id.into(),
            short_code: id.into(),
            flag: None,
            external_id: None,
        };
        let g1 = SingleGame {
            id: "M1".into(),
            kickoff: at(2026, 6, 11, 19),
            venue: None,
            group_id: "A".into(),
            home: TeamSlot { team_id: Some("AAA".into()), description: "A1".into() },
            away: TeamSlot { team_id: Some("BBB".into()), description: "A2".into() },
        };
        let g2 = SingleGame {
            id: "M2".into(),
            kickoff: at(2026, 6, 13, 19),
            venue: None,
            group_id: "A".into(),
            home: TeamSlot { team_id: Some("AAA".into()), description: "A1".into() },
            away: TeamSlot { team_id: Some("BBB".into()), description: "A2".into() },
        };
        let group = GroupGame {
            id: "A".into(),
            name: "A".into(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec!["M1".into(), "M2".into()]),
        };
        let t = Tournament {
            root: "A".into(),
            groups: HashMap::from([("A".to_string(), group)]),
            games: HashMap::from([("M1".to_string(), g1), ("M2".to_string(), g2)]),
            teams: HashMap::from([
                ("AAA".to_string(), team("AAA")),
                ("BBB".to_string(), team("BBB")),
            ]),
        };
        let result = Player {
            id: "result-user".into(),
            person_id: "p".into(),
            nick: "official".into(),
            full_name: "Official".into(),
            referrer: None,
            is_result_user: true,
            version: 0,
            match_predictions: vec![
                MatchPrediction { game_id: "M1".into(), home_score: 1, away_score: 0, locked: false },
                MatchPrediction { game_id: "M2".into(), home_score: 2, away_score: 0, locked: false },
            ],
            standings_predictions: vec![StandingsPrediction {
                group_id: "A".into(),
                ordering: vec!["AAA".into(), "BBB".into()],
                draw_order: vec!["AAA".into(), "BBB".into()],
                locked: false,
            }],
        };
        (t, result)
    }

    #[test]
    fn slice_keeps_only_played_matches() {
        let (t, result) = fixture();
        // Between the two kickoffs (after M1 + buffer, before M2): only M1 in.
        let now = at(2026, 6, 12, 12);
        let sliced = slice_result_as_of(&t, &result, now);
        assert_eq!(sliced.match_predictions.len(), 1);
        assert_eq!(sliced.match_predictions[0].game_id, "M1");
        // Group is not complete → standings dropped.
        assert!(sliced.standings_predictions.is_empty());
    }

    #[test]
    fn slice_keeps_everything_once_group_is_complete() {
        let (t, result) = fixture();
        let now = at(2026, 6, 20, 12); // after both kickoffs + buffer
        let sliced = slice_result_as_of(&t, &result, now);
        assert_eq!(sliced.match_predictions.len(), 2);
        assert_eq!(sliced.standings_predictions.len(), 1);
    }

    #[test]
    fn slice_is_noop_before_anything_is_played() {
        let (t, result) = fixture();
        let now = at(2026, 6, 1, 12); // before the tournament
        let sliced = slice_result_as_of(&t, &result, now);
        assert!(sliced.match_predictions.is_empty());
        assert!(sliced.standings_predictions.is_empty());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p api recompute::tests::slice_keeps_only_played_matches`
Expected: FAIL — `cannot find function slice_result_as_of`.

- [ ] **Step 3: Write the implementation**

In `crates/api/src/recompute.rs`, add the imports and the slice function. Change the top `use` block to include the needed domain types:

```rust
use chrono::{DateTime, Utc};
use domain::scoring::{score_tournament, ScoringConfig};
use domain::{GroupChildren, Player, Round, Tournament};
use storage::{Repository, Scoreboard};
```

Add these functions above `pub async fn recompute`:

```rust
/// Has a game been played as-of `now`? True once `now` passes
/// `kickoff + result_buffer(round)` — the inverse of `result_pending`.
fn game_played(t: &Tournament, game_id: &str, now: DateTime<Utc>) -> bool {
    t.games.get(game_id).is_some_and(|g| {
        let round = t
            .groups
            .get(&g.group_id)
            .map(|gr| gr.round)
            .unwrap_or(Round::GroupStage);
        now > g.kickoff + crate::timeflags::result_buffer(round)
    })
}

/// Are all of a leaf group's games played as-of `now`? Internal nodes (which do
/// not carry sliceable standings here) return false.
fn group_complete(t: &Tournament, group_id: &str, now: DateTime<Utc>) -> bool {
    match t.groups.get(group_id).map(|g| &g.children) {
        Some(GroupChildren::Games(ids)) => ids.iter().all(|id| game_played(t, id, now)),
        _ => false,
    }
}

/// Project the result-user onto "what has actually happened by `now`": keep
/// match results only for played games, and standings only for fully-played
/// groups. For real production entries this is a no-op (unplayed games carry no
/// entered result), so it is safe to apply unconditionally and makes the
/// materialised scoreboard/bracket correct as-of any clock.
fn slice_result_as_of(t: &Tournament, result: &Player, now: DateTime<Utc>) -> Player {
    let match_predictions = result
        .match_predictions
        .iter()
        .filter(|mp| game_played(t, &mp.game_id, now))
        .cloned()
        .collect();
    let standings_predictions = result
        .standings_predictions
        .iter()
        .filter(|sp| group_complete(t, &sp.group_id, now))
        .cloned()
        .collect();
    Player {
        match_predictions,
        standings_predictions,
        ..result.clone()
    }
}
```

Now use the slice inside `recompute`. Replace the body from the `// 1. Scoreboard` comment through the `resolve_bracket` call. Change:

```rust
    // 1. Scoreboard: score every real player vs the result user.
    let config = ScoringConfig::default();
    let mut scoreboard = Scoreboard::default();
    for player in &players {
        if player.is_result_user {
            continue;
        }
        let breakdown = score_tournament(&tournament, player, result_user, now, &config);
        scoreboard.entries.insert(player.id.clone(), breakdown);
    }
    repo.put_scoreboard(&scoreboard).await?;
```
to:
```rust
    // Project the result-user onto matches played as-of `now`, so the scoreboard
    // and bracket materialise correctly for the requested clock (no-op for real
    // post-result entries — see `slice_result_as_of`).
    let sliced = slice_result_as_of(&tournament, result_user, now);

    // 1. Scoreboard: score every real player vs the (sliced) result user.
    let config = ScoringConfig::default();
    let mut scoreboard = Scoreboard::default();
    for player in &players {
        if player.is_result_user {
            continue;
        }
        let breakdown = score_tournament(&tournament, player, &sliced, now, &config);
        scoreboard.entries.insert(player.id.clone(), breakdown);
    }
    repo.put_scoreboard(&scoreboard).await?;
```

And change the bracket-resolution line:
```rust
    let resolved = fwc26::resolve_bracket(&tournament, result_user);
```
to:
```rust
    let resolved = fwc26::resolve_bracket(&tournament, &sliced);
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p api recompute`
Expected: PASS (3 slice tests). Then run the whole api suite: `cargo test -p api` — Expected: PASS (existing recompute callers unaffected, since real entries make the slice a no-op).

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/recompute.rs
git commit -m "feat(api): recompute slices result-user to matches played as-of now"
```

---

## Task 10: `devRematerialize` mutation

**Files:**
- Modify: `crates/api/src/gql/mutation.rs`

- [ ] **Step 1: Write the failing test**

Add to a `#[cfg(test)]` module at the bottom of `crates/api/src/gql/mutation.rs` (create the module if none exists):

```rust
#[cfg(test)]
mod dev_rematerialize_tests {
    use super::*;

    #[test]
    fn dev_gate_blocks_when_stub_absent() {
        // The gate is a pure env check; assert its disabled branch directly.
        std::env::remove_var("LOCAL_AUTH_ISSUER");
        assert!(!dev_stub_enabled());
        std::env::set_var("LOCAL_AUTH_ISSUER", "local-dev");
        assert!(dev_stub_enabled());
        std::env::remove_var("LOCAL_AUTH_ISSUER");
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p api dev_rematerialize_tests`
Expected: FAIL — `cannot find function dev_stub_enabled`.

- [ ] **Step 3: Write the implementation**

In `crates/api/src/gql/mutation.rs`, add a helper near the top (after the `now` helper):

```rust
/// Whether the dev stub is enabled (same gate as the dev-login route / clock
/// override — the `LOCAL_AUTH_ISSUER` env var, absent in production).
fn dev_stub_enabled() -> bool {
    std::env::var("LOCAL_AUTH_ISSUER")
        .ok()
        .filter(|v| !v.is_empty())
        .is_some()
}
```

Add the mutation inside `impl MutationRoot` (after the admin `recompute` mutation):

```rust
    /// Dev-only: re-materialise the scoreboard + bracket as-of the request
    /// clock (`X-Dev-Now`). Unlike the admin `recompute`, this is gated on the
    /// dev stub rather than admin, so the dev-clock picker can call it as any
    /// logged-in dev player. Returns an error when the stub is disabled.
    async fn dev_rematerialize(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        if !dev_stub_enabled() {
            return Err(async_graphql::Error::new("dev rematerialize is disabled"));
        }
        let repo = repo(ctx);
        recompute(repo.as_ref(), now(ctx)).await.map_err(|e| {
            tracing::error!("dev_rematerialize failed: {e}");
            async_graphql::Error::new("rematerialize failed; please retry")
        })?;
        Ok(true)
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p api dev_rematerialize_tests`
Expected: PASS. Then `cargo test -p api` — Expected: PASS.

- [ ] **Step 5: Verify the schema exposes the mutation**

Run: `cargo build -p api`
Expected: builds clean. The async-graphql `#[Object]` macro auto-registers `devRematerialize` (camelCased) in the schema.

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/gql/mutation.rs
git commit -m "feat(api): dev-only devRematerialize mutation gated on the dev stub"
```

---

## Task 11: Hook the dev-clock picker to `devRematerialize`

**Files:**
- Modify: `web/src/graphql/queries.ts`
- Modify: `web/src/components/DevClock.tsx`

- [ ] **Step 1: Add the mutation document**

In `web/src/graphql/queries.ts`, add at the end of the file:

```ts
/** Dev-only: rebuild the scoreboard + bracket as-of the current dev clock. */
export const REMATERIALIZE_MUTATION = `
  mutation DevRematerialize {
    devRematerialize
  }
`
```

- [ ] **Step 2: Call it from the picker on apply + reset**

In `web/src/components/DevClock.tsx`:

Change the imports — add `useMutation` and the document:

```tsx
import { useMutation, useQuery } from 'urql'
```

and add to the existing `../graphql/queries` import:

```tsx
import { DEV_CLOCK_GAMES_QUERY, REMATERIALIZE_MUTATION } from '../graphql/queries'
```

Inside the `DevClock` component, after the `useQuery` line, add the mutation hook:

```tsx
  const [, rematerialize] = useMutation(REMATERIALIZE_MUTATION)
```

Replace the `apply` function with an async version that re-materialises before reloading:

```tsx
  // Apply only when BOTH a game and a phase are chosen, then re-materialise the
  // board as-of the new clock and reload. The mutation carries the freshly-set
  // X-Dev-Now header (client.ts reads it per request), so the server rebuilds
  // for the just-picked instant. Failures are swallowed — a prod build without
  // the dev mutation still reloads cleanly.
  const apply = async (g: string, p: '' | DevClockPhase) => {
    if (!g || !p) return
    const game = games.find((x) => x.id === g)
    if (!game) return
    setDevNow(devClockInstant(game.kickoff, p))
    try {
      await rematerialize({})
    } catch {
      /* ignore — reload regardless */
    }
    location.reload()
  }
```

Update the two `onChange` handlers to await `apply` (they already call it; make them async so the reload waits):

```tsx
  const onGame = (e: ChangeEvent<HTMLSelectElement>) => {
    setGameId(e.target.value)
    void apply(e.target.value, phase)
  }
  const onPhase = (e: ChangeEvent<HTMLSelectElement>) => {
    const p = e.target.value as '' | DevClockPhase
    setPhase(p)
    void apply(gameId, p)
  }
```

Replace the reset button's `onClick` with one that re-materialises against the real/default clock after clearing:

```tsx
            onClick={async () => {
              clearDevNow()
              try {
                await rematerialize({})
              } catch {
                /* ignore */
              }
              location.reload()
            }}
```

- [ ] **Step 3: Type-check + lint + build**

Run:
```bash
cd web && npm run lint && npm run build
```
Expected: no eslint errors; `tsc -b && vite build` succeeds.

- [ ] **Step 4: Commit**

```bash
git add web/src/graphql/queries.ts web/src/components/DevClock.tsx
git commit -m "feat(web): dev-clock picker re-materialises the scoreboard on every change"
```

---

## Task 12: E2E — scoreboard changes as the clock moves

**Files:**
- Create: `web/e2e/scenario-scoreboard.spec.ts`

- [ ] **Step 1: Write the E2E spec**

Create `web/e2e/scenario-scoreboard.spec.ts`:

```ts
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect, type Page } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * The scenario generator + as-of re-materialise loop, end to end. We seed the
 * `balanced` scenario into the e2e table (full results, ~12 players), then move
 * the dev-clock picker from early in the tournament to late and assert the
 * scoreboard total grows — proving the board re-materialises as-of the clock
 * from a single seed (the dev-clock picker fires `devRematerialize`).
 */

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '../..')

/** Sum of the visible scoreboard point cells. */
async function scoreboardTotal(page: Page): Promise<number> {
  const cells = page.locator('.scoreboard td.points, .scoreboard .total')
  const count = await cells.count()
  let total = 0
  for (let i = 0; i < count; i++) {
    const n = Number((await cells.nth(i).innerText()).replace(/[^\d-]/g, ''))
    if (!Number.isNaN(n)) total += n
  }
  return total
}

/** Pick a game + phase in the auth-bar dev clock; it applies + re-materialises. */
async function setClock(page: Page, gameId: string, phase: 'before' | 'after') {
  const selects = page.locator('.dev-clock select')
  await selects.nth(0).selectOption(gameId)
  await expect(selects.nth(1)).toBeEnabled()
  await selects.nth(1).selectOption(phase)
}

test.beforeAll(() => {
  // Seed the scenario into the same table the live stack booted (its name is
  // written by scripts/e2e-stack.sh to web/.e2e-table).
  const table = readFileSync(resolve(repoRoot, 'web/.e2e-table'), 'utf8').trim()
  execFileSync(
    'cargo',
    ['run', '-p', 'xtask', '--', 'scenario', 'balanced'],
    {
      cwd: repoRoot,
      stdio: 'inherit',
      env: {
        ...process.env,
        XPOOL_TABLE: table,
        DYNAMO_ENDPOINT: 'http://localhost:8001',
      },
    },
  )
})

test('scoreboard re-materialises larger as the dev clock advances', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  // Early: only the first match has been played → small (or zero) board.
  await setClock(page, 'M1', 'after')
  await page.getByRole('link', { name: 'Scoreboard' }).click()
  await expect(page.locator('.scoreboard')).toBeVisible()
  const early = await scoreboardTotal(page)

  // Late: a deep knockout game is "after" → many more matches scored.
  await setClock(page, 'M104', 'after') // a late knockout fixture
  await page.getByRole('link', { name: 'Scoreboard' }).click()
  await expect(page.locator('.scoreboard')).toBeVisible()
  const late = await scoreboardTotal(page)

  expect(late).toBeGreaterThan(early)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Confirm the scoreboard selectors + late game id**

Run:
```bash
grep -rn "scoreboard\|className=\"scoreboard\|points\|total" web/src/components | grep -i scoreboard | head
```
Expected: find the scoreboard component's class names. If the point cells use different class names than `.scoreboard td.points, .scoreboard .total`, update `scoreboardTotal`'s locator to match. Also confirm a late knockout game id exists:
```bash
jq -r '.games | keys[]' tournaments/fwc26.json | sort | tail
```
Pick a real late-stage game id (e.g. the Final) for the second `setClock` call if `M104` is not present.

- [ ] **Step 3: Run the E2E spec**

Run:
```bash
cd web && npm run e2e -- scenario-scoreboard
```
Expected: PASS. The suite boots the stack (global-setup), `beforeAll` seeds the scenario, and the test asserts the board grows. If it fails because the board is identical, verify the dev-clock picker actually fires `devRematerialize` (check the network panel assertions) and that `LOCAL_AUTH_ISSUER` is set in the e2e API env (it is, for dev-login).

- [ ] **Step 4: Commit**

```bash
git add web/e2e/scenario-scoreboard.spec.ts
git commit -m "test(e2e): scenario scoreboard re-materialises as the dev clock advances"
```

---

## Final verification

- [ ] **Step 1: Full Rust suite + clippy + fmt**

Run:
```bash
cargo fmt
cargo test --workspace
cargo clippy --workspace -- -D warnings
```
Expected: all green; clippy clean.

- [ ] **Step 2: Web build + lint**

Run:
```bash
cd web && npm run lint && npm run build
```
Expected: clean.

- [ ] **Step 3: Commit any fmt-only changes**

```bash
git add -A
git commit -m "chore(scenario): fmt" || echo "nothing to format"
```

---

## Self-review notes (spec coverage)

- **Ranking in a separate file, domain untouched** → Task 1.
- **Engine/policy split, coherence via reuse of `rank_group`/`resolve_bracket`** → Tasks 2–5; round-trip proof in Task 5.
- **3 scenarios, ~12 players, fixed whacky roster, deterministic seed** → Tasks 2, 3, 6.
- **`xtask scenario <id>`, reuse seeding machinery, switch = re-seed** → Tasks 7–8.
- **Seed full + `recompute_as_of` slice, unified into `recompute`** → Task 9.
- **Dev-only `devRematerialize` mutation gated like dev-login** → Task 10.
- **Dev-clock picker fires the mutation on change + reset** → Task 11.
- **E2E: seed + sweep clock + assert scoreboard changes** → Task 12.

**Deferred items from the spec, resolved here:** recompute stays in `crates/api` (the dev mutation is the materialiser; a freshly-seeded board is empty until the clock is moved once — acceptable, documented in the `xtask scenario` output). One shared pool (`pool-demo`) holds all 11 predictors. Ranking values are the plausible unique strengths in Task 1.
