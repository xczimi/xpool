//! DynamoDB `Repository` adapter. Single table, two key zones
//! (`DATA_MODEL.md` §9). Filled by task P3.

use crate::{Repository, Scoreboard};
use async_trait::async_trait;
use domain::{Identity, Motd, Person, Player, Pool, Tournament};

/// DynamoDB-backed repository. Scoped to `CURRENT_TOURNAMENT_ID` — all
/// per-tournament keys are prefixed with it.
#[derive(Clone)]
pub struct DynamoRepository {
    pub client: aws_sdk_dynamodb::Client,
    pub table: String,
    pub tournament_id: String,
}

impl DynamoRepository {
    /// Build from the environment. `DYNAMO_ENDPOINT` (DynamoDB Local) optional;
    /// `XPOOL_TABLE` (default `xpool`); `CURRENT_TOURNAMENT_ID` (default `fwc26`).
    pub async fn from_env() -> anyhow::Result<Self> {
        todo!("P3: build aws_config + client, read env")
    }
}

#[async_trait]
impl Repository for DynamoRepository {
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
