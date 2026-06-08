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
        (
            "whacky-onenil",
            "onenil",
            "Mr. One-Nil",
            PolicyKind::AlwaysHome,
        ),
        (
            "whacky-draw",
            "stalemate",
            "Sir Stalemate",
            PolicyKind::AlwaysDraw,
        ),
        ("whacky-chaos", "chaos", "Captain Chaos", PolicyKind::Chaos),
        (
            "whacky-homer",
            "homer",
            "The Homer",
            PolicyKind::Homer { fav: "BRA".into() },
        ),
        ("whacky-chalk", "formguide", "Form Guide", PolicyKind::Chalk),
    ];
    players.extend(
        whacky
            .into_iter()
            .map(|(id, nick, name, policy)| PlayerSpec {
                id: id.into(),
                nick: nick.into(),
                full_name: name.into(),
                policy,
                preexisting: false,
            }),
    );
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
pub fn build_policy(
    kind: &PolicyKind,
    scenario_id: &str,
    player_id: &str,
) -> Box<dyn ScorelinePolicy> {
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
        let scenario = scenarios()
            .into_iter()
            .find(|s| s.id == "balanced")
            .unwrap();
        let onenil = scenario
            .players
            .iter()
            .find(|p| p.id == "whacky-onenil")
            .unwrap();
        let out = player_outcome(&t, &r, &scenario, onenil);
        // Every group-stage game is 1-0 (knockout draws get an advancer but the
        // 90-min score AlwaysHome emits is still 1-0).
        assert!(out
            .match_predictions
            .iter()
            .all(|mp| (mp.home_score, mp.away_score) == (1, 0)));
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
