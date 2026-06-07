//! Persistence — the `Repository` trait and its adapters (`DATA_MODEL.md` §9).
//!
//! The trait and `Scoreboard` are a **locked contract**. The `InMemoryRepository`
//! and `DynamoRepository` bodies are filled by the `storage` subagent (task P3).

use async_trait::async_trait;
use domain::{Identity, Person, Player, PlayerId, Pool, Round, Tournament};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod dynamo;
pub mod memory;

pub use dynamo::DynamoRepository;
pub use memory::InMemoryRepository;

/// Materialized scoreboard — `playerId → {round → points}` (`SCORING.md` §8).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scoreboard {
    pub entries: HashMap<PlayerId, HashMap<Round, i64>>,
}

/// Persistence abstraction. One DynamoDB table behind it; an in-memory fake for
/// tests. `put_player` enforces optimistic concurrency on `Player::version`:
/// the repository owns the counter and bumps it on each write.
#[async_trait]
pub trait Repository: Send + Sync {
    async fn get_tournament(&self) -> anyhow::Result<Option<Tournament>>;
    async fn put_tournament(&self, t: &Tournament) -> anyhow::Result<()>;

    async fn get_player(&self, id: &str) -> anyhow::Result<Option<Player>>;
    async fn list_players(&self) -> anyhow::Result<Vec<Player>>;
    /// Find the player linked to a `Person` (i.e. whose `person_id` matches).
    /// `Player.id` and `Player.person_id` are distinct, so login resolution
    /// must look players up by person id through this method — not `get_player`,
    /// which is keyed by `Player.id`. Returns `None` if no player is linked.
    async fn get_player_by_person(&self, person_id: &str) -> anyhow::Result<Option<Player>>;
    /// Optimistic concurrency: pass the `Player` with the `version` it was
    /// last read at. Fails if the stored `version` no longer matches; on
    /// success the repository persists the player at `version + 1`. The
    /// caller does **not** bump `version`.
    async fn put_player(&self, p: &Player) -> anyhow::Result<()>;

    async fn get_scoreboard(&self) -> anyhow::Result<Option<Scoreboard>>;
    async fn put_scoreboard(&self, s: &Scoreboard) -> anyhow::Result<()>;

    async fn list_pools(&self) -> anyhow::Result<Vec<Pool>>;
    async fn put_pool(&self, p: &Pool) -> anyhow::Result<()>;
    /// Remove a pool. A no-op if no pool with `id` exists.
    async fn delete_pool(&self, id: &str) -> anyhow::Result<()>;

    async fn get_identity(
        &self,
        provider: &str,
        provider_id: &str,
    ) -> anyhow::Result<Option<Identity>>;
    async fn put_identity(&self, i: &Identity) -> anyhow::Result<()>;

    async fn get_person(&self, id: &str) -> anyhow::Result<Option<Person>>;
    async fn put_person(&self, p: &Person) -> anyhow::Result<()>;

    /// Return every `Identity` whose `verified_email` matches `email`.
    ///
    /// Use case: when a user signs in with a new provider, look up any existing
    /// identities that share the same verified e-mail address so the caller can
    /// link them to the same `Person`.
    async fn find_identities_by_verified_email(&self, email: &str)
        -> anyhow::Result<Vec<Identity>>;

    /// Atomically mark a single-use invite code as claimed. Returns `true`
    /// when this caller successfully claimed it (first time), `false` when
    /// it was already claimed (any prior or concurrent claim).
    ///
    /// The "claimed codes" set is a global key zone — invite codes are
    /// global (no tournament prefix). Codes for multi-use invites should
    /// NEVER be passed here; this is the SingleUse enforcement point.
    async fn claim_invite_code(&self, code: &str) -> anyhow::Result<bool>;
}
