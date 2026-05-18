//! DynamoDB `Repository` adapter (task P3).
//!
//! ## Key design (`DATA_MODEL.md` §9)
//!
//! Single table, two zones. We use a flat single-key design: **`pk`** is the
//! only key attribute — no sort key. Items that live in the same "namespace"
//! (e.g., all players in a tournament) share a common `pk` prefix, so the
//! caller can `Scan` with a filter expression when listing. This keeps the
//! schema as simple as possible while staying correct for the access patterns
//! in §9.
//!
//! | Item | `pk` | `data` |
//! |---|---|---|
//! | Person | `PERSON#<id>` | JSON |
//! | Identity | `IDENTITY#<provider>#<providerId>` | JSON |
//! | Player | `<t>#PLAYER#<playerId>` | JSON |
//! | Tournament | `<t>#TOURNAMENT` | JSON |
//! | Pool | `<t>#POOL#<poolId>` | JSON |
//! | Scoreboard | `<t>#SCOREBOARD` | JSON |
//!
//! Players additionally store a bare numeric `version` attribute so that a
//! DynamoDB conditional expression (`version = :v`) can guard `put_player`
//! without deserialising the full `data` blob.

use crate::{Repository, Scoreboard};
use anyhow::Context;
use async_trait::async_trait;
use aws_sdk_dynamodb::{
    error::SdkError,
    operation::put_item::PutItemError,
    types::{
        AttributeDefinition, AttributeValue, BillingMode, KeySchemaElement, KeyType,
        ScalarAttributeType,
    },
    Client,
};
use domain::{Identity, Person, Player, Pool, Tournament};

/// DynamoDB-backed repository. Scoped to `tournament_id` — every per-tournament
/// key is prefixed `<tournament_id>#…`.
#[derive(Clone)]
pub struct DynamoRepository {
    pub client: Client,
    pub table: String,
    pub tournament_id: String,
}

impl DynamoRepository {
    /// Build from environment.
    ///
    /// | Env var | Purpose | Default |
    /// |---|---|---|
    /// | `DYNAMO_ENDPOINT` | Custom endpoint (e.g., `http://localhost:8000` for DynamoDB Local) | SDK default (real AWS) |
    /// | `XPOOL_TABLE` | Table name | `xpool` |
    /// | `CURRENT_TOURNAMENT_ID` | Tournament namespace | `fwc26` |
    pub async fn from_env() -> anyhow::Result<Self> {
        let table = std::env::var("XPOOL_TABLE").unwrap_or_else(|_| "xpool".to_owned());
        let tournament_id =
            std::env::var("CURRENT_TOURNAMENT_ID").unwrap_or_else(|_| "fwc26".to_owned());

        let mut loader = aws_config::from_env();

        if let Ok(endpoint) = std::env::var("DYNAMO_ENDPOINT") {
            // Local mode (DynamoDB Local): supply a region and dummy static
            // credentials so the dev does not need any AWS env vars set.
            loader = loader
                .endpoint_url(endpoint)
                .region(aws_sdk_dynamodb::config::Region::new("us-east-1"))
                .credentials_provider(aws_sdk_dynamodb::config::Credentials::new(
                    "local", "local", None, None, "xpool-local",
                ));
        }

        let config = loader.load().await;
        let client = Client::new(&config);

        Ok(Self {
            client,
            table,
            tournament_id,
        })
    }

    /// Create the table if it does not already exist (on-demand billing).
    /// Idempotent — silently succeeds if the table exists.
    pub async fn ensure_table(&self) -> anyhow::Result<()> {
        let result = self
            .client
            .create_table()
            .table_name(&self.table)
            .billing_mode(BillingMode::PayPerRequest)
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("pk")
                    .attribute_type(ScalarAttributeType::S)
                    .build()
                    .unwrap(),
            )
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("pk")
                    .key_type(KeyType::Hash)
                    .build()
                    .unwrap(),
            )
            .send()
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(SdkError::ServiceError(e)) if e.err().is_resource_in_use_exception() => {
                Ok(()) // table already exists — idempotent
            }
            Err(e) => Err(e).context("create_table failed"),
        }
    }

    // ── Helpers ────────────────────────────────────────────────────────────

    /// Per-tournament key prefix.
    fn t(&self) -> &str {
        &self.tournament_id
    }

    /// Get a single item by `pk` and deserialise its `data` attribute.
    async fn get_item<T: serde::de::DeserializeOwned>(
        &self,
        pk: &str,
    ) -> anyhow::Result<Option<T>> {
        let resp = self
            .client
            .get_item()
            .table_name(&self.table)
            .key("pk", AttributeValue::S(pk.to_owned()))
            .send()
            .await
            .with_context(|| format!("get_item pk={pk}"))?;

        match resp.item {
            None => Ok(None),
            Some(item) => {
                let data = item
                    .get("data")
                    .and_then(|v| v.as_s().ok())
                    .with_context(|| format!("missing `data` attribute for pk={pk}"))?;
                let value =
                    serde_json::from_str(data).with_context(|| format!("deserialise pk={pk}"))?;
                Ok(Some(value))
            }
        }
    }

    /// Put a single item with a `pk` and JSON-serialised `data` attribute.
    async fn put_item_simple<T: serde::Serialize>(
        &self,
        pk: &str,
        value: &T,
    ) -> anyhow::Result<()> {
        let data = serde_json::to_string(value)?;
        self.client
            .put_item()
            .table_name(&self.table)
            .item("pk", AttributeValue::S(pk.to_owned()))
            .item("data", AttributeValue::S(data))
            .send()
            .await
            .with_context(|| format!("put_item pk={pk}"))?;
        Ok(())
    }

    /// Delete an item by `pk`. A no-op if the item does not exist.
    async fn delete_item(&self, pk: &str) -> anyhow::Result<()> {
        self.client
            .delete_item()
            .table_name(&self.table)
            .key("pk", AttributeValue::S(pk.to_owned()))
            .send()
            .await
            .with_context(|| format!("delete_item pk={pk}"))?;
        Ok(())
    }

    /// Scan items whose `pk` begins with `prefix` and deserialise them.
    async fn scan_prefix<T: serde::de::DeserializeOwned>(
        &self,
        prefix: &str,
    ) -> anyhow::Result<Vec<T>> {
        let resp = self
            .client
            .scan()
            .table_name(&self.table)
            .filter_expression("begins_with(pk, :pfx)")
            .expression_attribute_values(":pfx", AttributeValue::S(prefix.to_owned()))
            .send()
            .await
            .with_context(|| format!("scan prefix={prefix}"))?;

        let mut results = Vec::new();
        for item in resp.items.unwrap_or_default() {
            let data = item
                .get("data")
                .and_then(|v| v.as_s().ok())
                .context("missing `data` attribute in scan result")?;
            let value: T = serde_json::from_str(data).context("deserialise scan item")?;
            results.push(value);
        }
        Ok(results)
    }
}

#[async_trait]
impl Repository for DynamoRepository {
    // ── Tournament ─────────────────────────────────────────────────────────

    async fn get_tournament(&self) -> anyhow::Result<Option<Tournament>> {
        let pk = format!("{}#TOURNAMENT", self.t());
        self.get_item(&pk).await
    }

    async fn put_tournament(&self, t: &Tournament) -> anyhow::Result<()> {
        let pk = format!("{}#TOURNAMENT", self.t());
        self.put_item_simple(&pk, t).await
    }

    // ── Player ─────────────────────────────────────────────────────────────

    async fn get_player(&self, id: &str) -> anyhow::Result<Option<Player>> {
        let pk = format!("{}#PLAYER#{}", self.t(), id);
        self.get_item(&pk).await
    }

    async fn list_players(&self) -> anyhow::Result<Vec<Player>> {
        let prefix = format!("{}#PLAYER#", self.t());
        self.scan_prefix(&prefix).await
    }

    /// Optimistic concurrency via a DynamoDB conditional write.
    ///
    /// For a **new** player (no item in the table) the write uses
    /// `attribute_not_exists(pk)` — it will fail if the item already exists,
    /// protecting against lost-update races on first write.
    ///
    /// For an **existing** player the write uses
    /// `#ver = :v` — it fails if the stored `version` no longer matches,
    /// which is the standard OCC check.
    ///
    /// The caller is responsible for bumping `Player::version` before calling
    /// this method. On a conflict `anyhow::Error` is returned with a message
    /// containing "ConditionalCheckFailed".
    async fn put_player(&self, p: &Player) -> anyhow::Result<()> {
        let pk = format!("{}#PLAYER#{}", self.t(), p.id);
        let data = serde_json::to_string(p)?;

        // Check whether the item already exists to decide which condition to use.
        let existing = self
            .client
            .get_item()
            .table_name(&self.table)
            .key("pk", AttributeValue::S(pk.clone()))
            .projection_expression("#ver")
            .expression_attribute_names("#ver", "version")
            .send()
            .await
            .context("get_item for put_player condition check")?;

        let put = self
            .client
            .put_item()
            .table_name(&self.table)
            .item("pk", AttributeValue::S(pk.clone()))
            .item("data", AttributeValue::S(data))
            .item("version", AttributeValue::N(p.version.to_string()));

        let put = if existing.item.is_none() {
            // First write: only succeed if the item truly does not exist yet.
            put.condition_expression("attribute_not_exists(pk)")
        } else {
            // Subsequent write: succeed only if stored version matches.
            put.condition_expression("#ver = :v")
                .expression_attribute_names("#ver", "version")
                .expression_attribute_values(":v", AttributeValue::N(p.version.to_string()))
        };

        put.send().await.map_err(|e| match e {
            SdkError::ServiceError(ref se) => {
                if let PutItemError::ConditionalCheckFailedException(_) = se.err() {
                    anyhow::anyhow!(
                        "optimistic concurrency conflict for player {}: ConditionalCheckFailed",
                        p.id
                    )
                } else {
                    anyhow::anyhow!("put_player pk={pk}: {e}")
                }
            }
            other => anyhow::anyhow!("put_player pk={pk}: {other}"),
        })?;

        Ok(())
    }

    // ── Scoreboard ─────────────────────────────────────────────────────────

    async fn get_scoreboard(&self) -> anyhow::Result<Option<Scoreboard>> {
        let pk = format!("{}#SCOREBOARD", self.t());
        self.get_item(&pk).await
    }

    async fn put_scoreboard(&self, s: &Scoreboard) -> anyhow::Result<()> {
        let pk = format!("{}#SCOREBOARD", self.t());
        self.put_item_simple(&pk, s).await
    }

    // ── Pool ───────────────────────────────────────────────────────────────

    async fn list_pools(&self) -> anyhow::Result<Vec<Pool>> {
        let prefix = format!("{}#POOL#", self.t());
        self.scan_prefix(&prefix).await
    }

    async fn put_pool(&self, p: &Pool) -> anyhow::Result<()> {
        let pk = format!("{}#POOL#{}", self.t(), p.id);
        self.put_item_simple(&pk, p).await
    }

    async fn delete_pool(&self, id: &str) -> anyhow::Result<()> {
        let pk = format!("{}#POOL#{}", self.t(), id);
        self.delete_item(&pk).await
    }

    // ── Identity ───────────────────────────────────────────────────────────

    async fn get_identity(
        &self,
        provider: &str,
        provider_id: &str,
    ) -> anyhow::Result<Option<Identity>> {
        let pk = format!("IDENTITY#{provider}#{provider_id}");
        self.get_item(&pk).await
    }

    async fn put_identity(&self, i: &Identity) -> anyhow::Result<()> {
        let pk = format!("IDENTITY#{}#{}", i.provider, i.provider_id);
        self.put_item_simple(&pk, i).await
    }

    // ── Person ─────────────────────────────────────────────────────────────

    async fn get_person(&self, id: &str) -> anyhow::Result<Option<Person>> {
        let pk = format!("PERSON#{id}");
        self.get_item(&pk).await
    }

    async fn put_person(&self, p: &Person) -> anyhow::Result<()> {
        let pk = format!("PERSON#{}", p.id);
        self.put_item_simple(&pk, p).await
    }
}
