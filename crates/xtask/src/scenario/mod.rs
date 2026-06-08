//! Scenario test-data generator (see
//! `docs/superpowers/specs/2026-06-07-scenario-test-data-generator-design.md`).

pub mod engine;
pub mod policy;
pub mod ranking;
pub mod scenarios;

use crate::scenario::ranking::Ranking;
use crate::scenario::scenarios::{player_outcome, result_outcome, scenarios, Scenario};
use crate::seed::{fresh_player, put_player_idempotent, seed, RESULT_USER_ID};
use domain::{Identity, Person, Player};
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
            apply_outcome(
                repo,
                &player.id,
                out.match_predictions,
                out.standings_predictions,
            )
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
    if let Some(mut pool) = repo
        .list_pools()
        .await?
        .into_iter()
        .find(|p| p.id == "pool-demo")
    {
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
        let rpath =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tournaments/fwc26-rankings.json");
        seed_scenario(&repo, scenario, &rpath).await.unwrap();
        repo
    }

    #[tokio::test]
    async fn result_user_and_players_get_predictions() {
        let repo = seeded_repo("balanced").await;

        let result = repo.get_player(RESULT_USER_ID).await.unwrap().unwrap();
        assert!(
            !result.match_predictions.is_empty(),
            "result-user has results"
        );
        assert!(result.is_result_user);

        let ada = repo.get_player("demo-ada").await.unwrap().unwrap();
        assert!(
            !ada.match_predictions.is_empty(),
            "demo player has predictions"
        );

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

        let pool = repo
            .list_pools()
            .await
            .unwrap()
            .into_iter()
            .find(|p| p.id == "pool-demo")
            .unwrap();
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
        let rpath =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tournaments/fwc26-rankings.json");
        let err = seed_scenario(&repo, "nope", &rpath).await.unwrap_err();
        assert!(err.to_string().contains("unknown scenario"));
    }
}
