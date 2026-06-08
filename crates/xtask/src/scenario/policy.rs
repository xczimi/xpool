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
        (self.rng.random_range(0..=4), self.rng.random_range(0..=4))
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
        let mut p = AlwaysHome;
        assert_eq!(p.score(&ctx("A", 1, "B", 9)), (1, 0));
    }

    #[test]
    fn always_draw_is_one_one() {
        let mut p = AlwaysDraw;
        assert_eq!(p.score(&ctx("A", 1, "B", 9)), (1, 1));
    }

    #[test]
    fn chalk_favours_the_stronger_side_and_never_upsets() {
        let mut p = Chalk;
        assert_eq!(p.score(&ctx("A", 9, "B", 1)), (1, 0)); // home stronger
        assert_eq!(p.score(&ctx("A", 1, "B", 9)), (0, 1)); // away stronger
        assert_eq!(p.score(&ctx("A", 5, "B", 5)), (1, 0)); // tie → home
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
        assert_ne!(
            seed_for("chalk", "demo-ada"),
            seed_for("chalk", "demo-alan")
        );
        assert_ne!(seed_for("chalk", "demo-ada"), seed_for("chaos", "demo-ada"));
    }
}
