//! Demo data seeding for local dev (`xtask seed`).
//!
//! Idempotent: every entity has a fixed id, so re-running overwrites rather
//! than duplicates. Creates a result-user player, ~6 demo players (each with a
//! Person + Identity), and one demo Pool.

use domain::{Identity, Person, Player, Pool};
use storage::Repository;

/// The fixed id of the result-user player (`is_result_user = true`).
pub const RESULT_USER_ID: &str = "result-user";

/// Demo player definitions: `(player_id, nick, full_name)`.
const DEMO_PLAYERS: [(&str, &str, &str); 6] = [
    ("demo-ada", "ada", "Ada Lovelace"),
    ("demo-alan", "alan", "Alan Turing"),
    ("demo-grace", "grace", "Grace Hopper"),
    ("demo-linus", "linus", "Linus Torvalds"),
    ("demo-margaret", "margaret", "Margaret Hamilton"),
    ("demo-dennis", "dennis", "Dennis Ritchie"),
];

fn fresh_player(id: &str, person_id: &str, nick: &str, full_name: &str, is_result: bool) -> Player {
    Player {
        id: id.to_owned(),
        person_id: person_id.to_owned(),
        nick: nick.to_owned(),
        full_name: full_name.to_owned(),
        referrer: None,
        is_result_user: is_result,
        version: 0,
        match_predictions: Vec::new(),
        standings_predictions: Vec::new(),
    }
}

/// Put a player idempotently — preserves the stored `version` so the
/// optimistic-concurrency guard does not reject the re-seed.
async fn put_player_idempotent(repo: &dyn Repository, mut player: Player) -> anyhow::Result<()> {
    if let Some(existing) = repo.get_player(&player.id).await? {
        player.version = existing.version;
    }
    repo.put_player(&player).await
}

/// Seed demo data into the repository. Idempotent.
pub async fn seed(repo: &dyn Repository) -> anyhow::Result<()> {
    // Result user — its prediction set is the official result (DATA_MODEL §5).
    let result_person = Person {
        id: "person-result".to_owned(),
        identity_ids: Vec::new(),
    };
    repo.put_person(&result_person).await?;
    put_player_idempotent(
        repo,
        fresh_player(
            RESULT_USER_ID,
            &result_person.id,
            "official",
            "Official Result",
            true,
        ),
    )
    .await?;

    // Demo players, each with a Person + email Identity.
    // The identity is keyed at ("email", "{player_id}@dev.invalid") so that
    // `dev_login` — which mints JWTs with `email = "{id}@dev.invalid"` — routes
    // through `identity_key_for("dev" connection) → ("email", email)` and finds
    // this row, resolving to the Player rather than AuthenticatedUnclaimed.
    for (player_id, nick, full_name) in DEMO_PLAYERS {
        let person_id = format!("person-{nick}");
        let identity_id = format!("identity-{nick}");
        let dev_email = format!("{player_id}@dev.invalid");

        let identity = Identity {
            id: identity_id.clone(),
            provider: "email".to_owned(),
            provider_id: dev_email.clone(),
            person_id: person_id.clone(),
            verified_email: Some(dev_email),
        };
        repo.put_identity(&identity).await?;

        let person = Person {
            id: person_id.clone(),
            identity_ids: vec![identity_id],
        };
        repo.put_person(&person).await?;

        put_player_idempotent(
            repo,
            fresh_player(player_id, &person_id, nick, full_name, false),
        )
        .await?;
    }

    // One demo pool owned by the first demo player, with all demo players.
    // The join code is fixed so end-to-end tests can rely on it.
    let pool = Pool {
        id: "pool-demo".to_owned(),
        name: "Demo Pool".to_owned(),
        owner: DEMO_PLAYERS[0].0.to_owned(),
        members: DEMO_PLAYERS
            .iter()
            .map(|(id, _, _)| id.to_string())
            .collect(),
        join_code: "DEMOPOOL".to_owned(),
    };
    repo.put_pool(&pool).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage::InMemoryRepository;

    #[tokio::test]
    async fn demo_identities_are_resolvable_by_dev_login() {
        let repo = InMemoryRepository::new();
        seed(&repo).await.expect("seed failed");

        for (player_id, nick, _) in DEMO_PLAYERS {
            let expected_email = format!("{player_id}@dev.invalid");
            // `dev_login` resolves via identity_key_for("dev") → ("email", email).
            // Verify the identity row keyed at ("email", email) exists and has
            // verified_email set.
            let identity = repo
                .get_identity("email", &expected_email)
                .await
                .expect("repo error")
                .unwrap_or_else(|| panic!("no identity row for {nick} at ({expected_email})"));

            assert_eq!(
                identity.verified_email.as_deref(),
                Some(expected_email.as_str()),
                "verified_email mismatch for {nick}"
            );
            // Person row must exist so the resolver can find the player.
            let person_id = format!("person-{nick}");
            assert!(
                repo.get_person(&person_id).await.expect("repo error").is_some(),
                "missing Person row for {nick}"
            );
        }
    }
}
