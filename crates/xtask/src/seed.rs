//! Demo data seeding for local dev (`xtask seed`).
//!
//! Idempotent: every entity has a fixed id, so re-running overwrites rather
//! than duplicates. Creates a result-user player, ~6 demo players (each with a
//! Person + Identity), and one demo Pool.

use domain::{Identity, Invite, Person, Player, Pool};
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

pub(crate) fn fresh_player(
    id: &str,
    person_id: &str,
    nick: &str,
    full_name: &str,
    is_result: bool,
) -> Player {
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
pub(crate) async fn put_player_idempotent(
    repo: &dyn Repository,
    mut player: Player,
) -> anyhow::Result<()> {
    if let Some(existing) = repo.get_player(&player.id).await? {
        player.version = existing.version;
    }
    repo.put_player(&player).await
}

/// Inner seed implementation that takes the result-user email explicitly.
/// Public callers use [`seed`] which resolves the email from the environment.
///
/// The result-user's Identity is keyed by email (`IDENTITY#email#<email>`).
/// If `RESULT_USER_EMAIL` is changed between runs the old key at the previous
/// email address will linger as an orphan — the trait has no `delete_identity`
/// method and adding one just to handle this rare operator action was judged
/// disproportionate.  Operators who rotate the email can remove the stale row
/// manually (e.g. via the AWS console or `aws dynamodb delete-item`).
async fn seed_with_email(repo: &dyn Repository, result_user_email: String) -> anyhow::Result<()> {
    // Result user — its prediction set is the official result (DATA_MODEL §5).
    let result_person = Person {
        id: "person-result".to_owned(),
        identity_ids: vec!["identity-result-user".to_owned()],
    };
    repo.put_person(&result_person).await?;

    // The Identity row is what allows the operator to log in as admin.
    // `put_identity` is last-write-wins on (`email`, <email>), so re-seeding
    // with a new RESULT_USER_EMAIL updates the row atomically.
    let result_identity = Identity {
        id: "identity-result-user".to_owned(),
        provider: "email".to_owned(),
        provider_id: result_user_email.clone(),
        person_id: result_person.id.clone(),
        verified_email: Some(result_user_email),
    };
    repo.put_identity(&result_identity).await?;

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
    // The referral graph: every demo player is a "founder" invited directly by
    // the result-user (the graph root), so each is an admin who may create pools
    // (`may_create_pool`). The pool members they later invite are normal players
    // whose referrer points at the inviting member, not the result-user.
    let admin_id = DEMO_PLAYERS[0].0;
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

        let mut player = fresh_player(player_id, &person_id, nick, full_name, false);
        player.referrer = Some(RESULT_USER_ID.to_owned());
        put_player_idempotent(repo, player).await?;
    }

    // One demo pool owned by the admin (first demo player), with all demo
    // players. The prefix and the owner's invite code are fixed so end-to-end
    // tests and manual dev can rely on the link `DEMO-DEMP7K42AB`.
    let pool = Pool {
        id: "pool-demo".to_owned(),
        name: "Demo Pool".to_owned(),
        owner: admin_id.to_owned(),
        members: DEMO_PLAYERS
            .iter()
            .map(|(id, _, _)| id.to_string())
            .collect(),
        prefix: "DEMO".to_owned(),
    };
    repo.put_pool(&pool).await?;

    // The owner's invite row — the pool link (a bare `DEMO` prefix resolves here).
    let owner_invite = Invite {
        code: "DEMP7K42AB".to_owned(),
        pool_id: pool.id.clone(),
        invited_by: admin_id.to_owned(),
        created_at: chrono::Utc::now(),
        expires_at: None,
        revoked: false,
    };
    repo.put_invite(&owner_invite).await?;

    Ok(())
}

/// Seed demo data into the repository. Idempotent.
///
/// Reads `RESULT_USER_EMAIL` from the environment; defaults to
/// `result-user@dev.invalid` when the variable is absent.
pub async fn seed(repo: &dyn Repository) -> anyhow::Result<()> {
    // Result-user email is configurable so the operator's real verified email
    // can be set in production without touching code.  Defaults to a synthetic
    // address that no one can authenticate with, effectively disabling admin.
    let result_user_email =
        std::env::var("RESULT_USER_EMAIL").unwrap_or_else(|_| "result-user@dev.invalid".into());
    seed_with_email(repo, result_user_email).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use storage::InMemoryRepository;

    #[tokio::test]
    async fn demo_identities_are_resolvable_by_dev_login() {
        let repo = InMemoryRepository::new();
        seed_with_email(&repo, "result-user@dev.invalid".into())
            .await
            .expect("seed failed");

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
                repo.get_person(&person_id)
                    .await
                    .expect("repo error")
                    .is_some(),
                "missing Person row for {nick}"
            );

            // Full resolution chain: identity.person_id → Player. The resolver
            // looks the player up by person id (Player.id != Player.person_id),
            // so this must return the seeded player — not None.
            let player = repo
                .get_player_by_person(&identity.person_id)
                .await
                .expect("repo error")
                .unwrap_or_else(|| panic!("person {person_id} resolves to no player for {nick}"));
            assert_eq!(
                player.id, player_id,
                "person {person_id} resolved to the wrong player"
            );
        }
    }

    #[tokio::test]
    async fn result_user_identity_defaults_to_dev_invalid() {
        let repo = InMemoryRepository::new();
        seed_with_email(&repo, "result-user@dev.invalid".into())
            .await
            .expect("seed failed");

        let identity = repo
            .get_identity("email", "result-user@dev.invalid")
            .await
            .expect("repo error")
            .expect("result-user identity row missing");
        assert_eq!(identity.person_id, "person-result");
        assert_eq!(
            identity.verified_email.as_deref(),
            Some("result-user@dev.invalid")
        );

        // The admin must resolve through the full chain too: person → Player.
        let player = repo
            .get_player_by_person(&identity.person_id)
            .await
            .expect("repo error")
            .expect("result-user person resolves to no player");
        assert_eq!(player.id, RESULT_USER_ID);
        assert!(player.is_result_user);
    }

    #[tokio::test]
    async fn result_user_identity_uses_override_email_when_set() {
        let repo = InMemoryRepository::new();
        seed_with_email(&repo, "pool@xczimi.com".into())
            .await
            .expect("seed failed");

        let identity = repo
            .get_identity("email", "pool@xczimi.com")
            .await
            .expect("repo error")
            .expect("result-user identity row missing at overridden email");
        assert_eq!(identity.person_id, "person-result");
        assert_eq!(identity.verified_email.as_deref(), Some("pool@xczimi.com"));
        // Default address must NOT have a row — only the override exists.
        let old_row = repo
            .get_identity("email", "result-user@dev.invalid")
            .await
            .expect("repo error");
        assert!(
            old_row.is_none(),
            "stale default identity row should not exist when override is active"
        );
    }
}
