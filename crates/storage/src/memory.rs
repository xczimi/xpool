//! In-memory `Repository` fake for unit/integration tests. Filled by task P3.

use crate::{Repository, Scoreboard};
use async_trait::async_trait;
use domain::{Identity, Motd, Person, Player, Pool, Tournament};

/// Mutex-wrapped in-memory store. Cheap to clone (shares the inner state).
#[derive(Clone, Default)]
pub struct InMemoryRepository {
    // P3: inner Arc<Mutex<...>> state.
}

impl InMemoryRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Repository for InMemoryRepository {
    async fn get_tournament(&self) -> anyhow::Result<Option<Tournament>> {
        todo!("P3")
    }
    async fn put_tournament(&self, _t: &Tournament) -> anyhow::Result<()> {
        todo!("P3")
    }
    async fn get_player(&self, _id: &str) -> anyhow::Result<Option<Player>> {
        todo!("P3")
    }
    async fn list_players(&self) -> anyhow::Result<Vec<Player>> {
        todo!("P3")
    }
    async fn put_player(&self, _p: &Player) -> anyhow::Result<()> {
        todo!("P3")
    }
    async fn get_scoreboard(&self) -> anyhow::Result<Option<Scoreboard>> {
        todo!("P3")
    }
    async fn put_scoreboard(&self, _s: &Scoreboard) -> anyhow::Result<()> {
        todo!("P3")
    }
    async fn list_pools(&self) -> anyhow::Result<Vec<Pool>> {
        todo!("P3")
    }
    async fn put_pool(&self, _p: &Pool) -> anyhow::Result<()> {
        todo!("P3")
    }
    async fn get_motd(&self) -> anyhow::Result<Option<Motd>> {
        todo!("P3")
    }
    async fn put_motd(&self, _m: &Motd) -> anyhow::Result<()> {
        todo!("P3")
    }
    async fn get_identity(
        &self,
        _provider: &str,
        _provider_id: &str,
    ) -> anyhow::Result<Option<Identity>> {
        todo!("P3")
    }
    async fn put_identity(&self, _i: &Identity) -> anyhow::Result<()> {
        todo!("P3")
    }
    async fn get_person(&self, _id: &str) -> anyhow::Result<Option<Person>> {
        todo!("P3")
    }
    async fn put_person(&self, _p: &Person) -> anyhow::Result<()> {
        todo!("P3")
    }
}
