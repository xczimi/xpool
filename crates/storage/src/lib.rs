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
/// tests. `put_player` enforces optimistic concurrency on `Player::version`.
#[async_trait]
pub trait Repository: Send + Sync {
    async fn get_tournament(&self) -> anyhow::Result<Option<Tournament>>;
    async fn put_tournament(&self, t: &Tournament) -> anyhow::Result<()>;

    async fn get_player(&self, id: &str) -> anyhow::Result<Option<Player>>;
    async fn list_players(&self) -> anyhow::Result<Vec<Player>>;
    /// Fails if the stored `version` does not match `p.version` (the caller
    /// must bump `version` on write).
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
}
