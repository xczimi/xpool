//! DynamoDB `Repository` adapter (task P3).
//!
//! ## Key design (`DATA_MODEL.md` §9)
//!
//! Single table with a composite key: **`pk`** (partition) + **`sk`** (sort).
//! Items that share a "namespace" (e.g., all players in a tournament) share a
//! `pk` and are distinguished by `sk`, so listing a namespace is a single
//! `Query` of one partition — not a table-wide `Scan`.
//!
//! Single-instance items (Person, Identity, Tournament, Scoreboard) have no
//! natural sibling set; they use a constant `sk` of `"#"`.
//!
//! | Item | `pk` | `sk` | `data` |
//! |---|---|---|---|
//! | Person | `PERSON#<id>` | `#` | JSON |
//! | Identity | `IDENTITY#<provider>#<providerId>` | `#` | JSON |
//! | Player | `<t>#PLAYER` | `<playerId>` | JSON |
//! | Tournament | `<t>#TOURNAMENT` | `#` | JSON |
//! | Pool | `<t>#POOL` | `<poolId>` | JSON |
//! | Scoreboard | `<t>#SCOREBOARD` | `#` | JSON |
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
        ScalarAttributeType, TableStatus,
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
    ///
    /// `create_table` returns while the table is still `CREATING`; an
    /// immediate `put_item`/`get_item` can then fail with
    /// `ResourceNotFoundException`. After issuing the create this polls
    /// `describe_table` until the table reports `TableStatus::Active`, so the
    /// repository is safe to use the moment `ensure_table` returns.
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
            .attribute_definitions(
                AttributeDefinition::builder()
                    .attribute_name("sk")
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
            .key_schema(
                KeySchemaElement::builder()
                    .attribute_name("sk")
                    .key_type(KeyType::Range)
                    .build()
                    .unwrap(),
            )
            .send()
            .await;

        match result {
            Ok(_) => {}
            Err(SdkError::ServiceError(e)) if e.err().is_resource_in_use_exception() => {
                // table already exists — idempotent
            }
            Err(e) => return Err(e).context("create_table failed"),
        }

        self.wait_for_active().await
    }

    /// Poll `describe_table` until the table reports `TableStatus::Active`.
    /// Bounded so a stuck table surfaces an error instead of hanging forever.
    async fn wait_for_active(&self) -> anyhow::Result<()> {
        // 60 attempts × 500 ms = 30 s ceiling. DynamoDB Local activates almost
        // instantly; real AWS typically settles within a few seconds.
        for _ in 0..60 {
            let desc = self
                .client
                .describe_table()
                .table_name(&self.table)
                .send()
                .await
                .with_context(|| format!("describe_table {}", self.table))?;

            let status = desc.table.and_then(|t| t.table_status);
            if status == Some(TableStatus::Active) {
                return Ok(());
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        anyhow::bail!(
            "table {} did not become ACTIVE within the timeout",
            self.table
        )
    }

    /// Delete the table. Used by e2e teardown. Idempotent — a missing table
    /// is treated as success.
    pub async fn delete_table(&self) -> anyhow::Result<()> {
        match self
            .client
            .delete_table()
            .table_name(&self.table)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(SdkError::ServiceError(e)) if e.err().is_resource_not_found_exception() => {
                Ok(()) // table already gone — idempotent
            }
            Err(e) => Err(e).context("delete_table failed"),
        }
    }

    // ── Helpers ────────────────────────────────────────────────────────────

    /// Per-tournament key prefix.
    fn t(&self) -> &str {
        &self.tournament_id
    }

    /// Get a single item by (`pk`, `sk`) and deserialise its `data` attribute.
    async fn get_item<T: serde::de::DeserializeOwned>(
        &self,
        pk: &str,
        sk: &str,
    ) -> anyhow::Result<Option<T>> {
        let resp = self
            .client
            .get_item()
            .table_name(&self.table)
            .key("pk", AttributeValue::S(pk.to_owned()))
            .key("sk", AttributeValue::S(sk.to_owned()))
            .send()
            .await
            .with_context(|| format!("get_item pk={pk} sk={sk}"))?;

        match resp.item {
            None => Ok(None),
            Some(item) => {
                let data = item
                    .get("data")
                    .and_then(|v| v.as_s().ok())
                    .with_context(|| format!("missing `data` attribute for pk={pk} sk={sk}"))?;
                let value = serde_json::from_str(data)
                    .with_context(|| format!("deserialise pk={pk} sk={sk}"))?;
                Ok(Some(value))
            }
        }
    }

    /// Put a single item with a (`pk`, `sk`) key and JSON-serialised `data`.
    async fn put_item_simple<T: serde::Serialize>(
        &self,
        pk: &str,
        sk: &str,
        value: &T,
    ) -> anyhow::Result<()> {
        let data = serde_json::to_string(value)?;
        self.client
            .put_item()
            .table_name(&self.table)
            .item("pk", AttributeValue::S(pk.to_owned()))
            .item("sk", AttributeValue::S(sk.to_owned()))
            .item("data", AttributeValue::S(data))
            .send()
            .await
            .with_context(|| format!("put_item pk={pk} sk={sk}"))?;
        Ok(())
    }

    /// Delete an item by (`pk`, `sk`). A no-op if the item does not exist.
    async fn delete_item(&self, pk: &str, sk: &str) -> anyhow::Result<()> {
        self.client
            .delete_item()
            .table_name(&self.table)
            .key("pk", AttributeValue::S(pk.to_owned()))
            .key("sk", AttributeValue::S(sk.to_owned()))
            .send()
            .await
            .with_context(|| format!("delete_item pk={pk} sk={sk}"))?;
        Ok(())
    }

    /// Query every item in the `pk` partition and deserialise them.
    ///
    /// DynamoDB returns at most 1 MB of data per `Query` call and sets
    /// `LastEvaluatedKey` when more items remain. This loops on
    /// `ExclusiveStartKey` until the query is exhausted, so callers always get
    /// the complete result set. Unlike a `Scan`, a `Query` reads only the one
    /// partition — it never touches items in other namespaces.
    async fn query_partition<T: serde::de::DeserializeOwned>(
        &self,
        pk: &str,
    ) -> anyhow::Result<Vec<T>> {
        let mut results = Vec::new();
        let mut last_evaluated_key = None;

        loop {
            let resp = self
                .client
                .query()
                .table_name(&self.table)
                .key_condition_expression("pk = :pk")
                .expression_attribute_values(":pk", AttributeValue::S(pk.to_owned()))
                .set_exclusive_start_key(last_evaluated_key.clone())
                .send()
                .await
                .with_context(|| format!("query pk={pk}"))?;

            for item in resp.items.unwrap_or_default() {
                let data = item
                    .get("data")
                    .and_then(|v| v.as_s().ok())
                    .context("missing `data` attribute in query result")?;
                let value: T = serde_json::from_str(data).context("deserialise query item")?;
                results.push(value);
            }

            // Continue until DynamoDB stops returning a continuation key.
            last_evaluated_key = resp.last_evaluated_key;
            if last_evaluated_key.is_none() {
                break;
            }
        }

        Ok(results)
    }
}

/// Sort key for single-instance items that have no natural sibling set
/// (Person, Identity, Tournament, Scoreboard). The composite-key table still
/// requires an `sk`; a constant keeps these items addressable by a fixed key.
const SINGLETON_SK: &str = "#";

#[async_trait]
impl Repository for DynamoRepository {
    // ── Tournament ─────────────────────────────────────────────────────────

    async fn get_tournament(&self) -> anyhow::Result<Option<Tournament>> {
        let pk = format!("{}#TOURNAMENT", self.t());
        self.get_item(&pk, SINGLETON_SK).await
    }

    async fn put_tournament(&self, t: &Tournament) -> anyhow::Result<()> {
        let pk = format!("{}#TOURNAMENT", self.t());
        self.put_item_simple(&pk, SINGLETON_SK, t).await
    }

    // ── Player ─────────────────────────────────────────────────────────────

    async fn get_player(&self, id: &str) -> anyhow::Result<Option<Player>> {
        let pk = format!("{}#PLAYER", self.t());
        self.get_item(&pk, id).await
    }

    async fn list_players(&self) -> anyhow::Result<Vec<Player>> {
        let pk = format!("{}#PLAYER", self.t());
        self.query_partition(&pk).await
    }

    /// Optimistic concurrency via a DynamoDB conditional write.
    ///
    /// The **repository** owns the `version` counter: the caller passes the
    /// `Player` exactly as it was last read (with the version it read), and
    /// the write is conditioned on the stored version still equalling that
    /// value. On success the item is persisted with `version + 1`.
    ///
    /// The write is a **single atomic conditional `put_item`** — no preceding
    /// `get_item`. The condition is `attribute_not_exists(pk) OR #ver = :v`
    /// where `:v` is the caller-supplied (old) version:
    ///
    /// - **New player**: no item exists, so `attribute_not_exists(pk)` holds
    ///   and the write succeeds, storing `version + 1` (a first write of a
    ///   version-0 player stores 1). A racing second insert of the same id
    ///   finds the item present and a version mismatch — both clauses fail —
    ///   so it is rejected.
    /// - **Existing player**: `#ver = :v` succeeds only if the stored version
    ///   still equals the version the caller last read. Two writers that both
    ///   read version `n` race: the first stores `n + 1`, the second's
    ///   condition fails.
    ///
    /// Because the condition is evaluated atomically with the write, the
    /// new-vs-update decision is condition-driven — never a stale read. On a
    /// conflict `anyhow::Error` is returned with a message containing
    /// "ConditionalCheckFailed".
    async fn put_player(&self, p: &Player) -> anyhow::Result<()> {
        let pk = format!("{}#PLAYER", self.t());
        let sk = p.id.clone();
        // Persist the player with the next version; the in-memory `data` blob
        // and the bare `version` attribute stay consistent.
        let next_version = p.version.saturating_add(1);
        let stored = Player {
            version: next_version,
            ..p.clone()
        };
        let data = serde_json::to_string(&stored)?;

        // One atomic conditional put: the item must either not yet exist
        // (first write) or still carry the version the caller last read
        // (in-place update). No prior `get_item` — the branch is the
        // condition itself, so it is evaluated atomically with the write.
        let put = self
            .client
            .put_item()
            .table_name(&self.table)
            .item("pk", AttributeValue::S(pk.clone()))
            .item("sk", AttributeValue::S(sk.clone()))
            .item("data", AttributeValue::S(data))
            .item("version", AttributeValue::N(next_version.to_string()))
            .condition_expression("attribute_not_exists(pk) OR #ver = :v")
            .expression_attribute_names("#ver", "version")
            .expression_attribute_values(":v", AttributeValue::N(p.version.to_string()));

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
        self.get_item(&pk, SINGLETON_SK).await
    }

    async fn put_scoreboard(&self, s: &Scoreboard) -> anyhow::Result<()> {
        let pk = format!("{}#SCOREBOARD", self.t());
        self.put_item_simple(&pk, SINGLETON_SK, s).await
    }

    // ── Pool ───────────────────────────────────────────────────────────────

    async fn list_pools(&self) -> anyhow::Result<Vec<Pool>> {
        let pk = format!("{}#POOL", self.t());
        self.query_partition(&pk).await
    }

    async fn put_pool(&self, p: &Pool) -> anyhow::Result<()> {
        let pk = format!("{}#POOL", self.t());
        self.put_item_simple(&pk, &p.id, p).await
    }

    async fn delete_pool(&self, id: &str) -> anyhow::Result<()> {
        let pk = format!("{}#POOL", self.t());
        self.delete_item(&pk, id).await
    }

    // ── Identity ───────────────────────────────────────────────────────────

    async fn get_identity(
        &self,
        provider: &str,
        provider_id: &str,
    ) -> anyhow::Result<Option<Identity>> {
        let pk = format!("IDENTITY#{provider}#{provider_id}");
        self.get_item(&pk, SINGLETON_SK).await
    }

    async fn put_identity(&self, i: &Identity) -> anyhow::Result<()> {
        let pk = format!("IDENTITY#{}#{}", i.provider, i.provider_id);
        self.put_item_simple(&pk, SINGLETON_SK, i).await
    }

    // ── Person ─────────────────────────────────────────────────────────────

    async fn get_person(&self, id: &str) -> anyhow::Result<Option<Person>> {
        let pk = format!("PERSON#{id}");
        self.get_item(&pk, SINGLETON_SK).await
    }

    async fn put_person(&self, p: &Person) -> anyhow::Result<()> {
        let pk = format!("PERSON#{}", p.id);
        self.put_item_simple(&pk, SINGLETON_SK, p).await
    }

    // ── Invite code usage ──────────────────────────────────────────────────

    /// Atomically mark a single-use invite code as claimed using a conditional
    /// `PutItem`. The item key is `INVITE_USED#<code>` (pk) / `#` (sk).
    ///
    /// `attribute_not_exists(pk)` succeeds only when no item with that pk
    /// exists yet — i.e., this is the first claim. A
    /// `ConditionalCheckFailedException` means the code was already claimed and
    /// we return `false`. Other errors propagate.
    async fn claim_invite_code(&self, code: &str) -> anyhow::Result<bool> {
        let pk = format!("INVITE_USED#{code}");
        let result = self
            .client
            .put_item()
            .table_name(&self.table)
            .item("pk", AttributeValue::S(pk.clone()))
            .item("sk", AttributeValue::S(SINGLETON_SK.to_owned()))
            .condition_expression("attribute_not_exists(pk)")
            .send()
            .await;

        match result {
            Ok(_) => Ok(true),
            Err(SdkError::ServiceError(ref se))
                if matches!(se.err(), PutItemError::ConditionalCheckFailedException(_)) =>
            {
                Ok(false)
            }
            Err(e) => Err(anyhow::anyhow!("claim_invite_code pk={pk}: {e}")),
        }
    }

    // ── Identity lookup ────────────────────────────────────────────────────

    /// Return every `Identity` whose `verified_email` matches `email`.
    ///
    /// # Scale note
    ///
    /// Linear scan of the identity partition. With ~hundreds of identities at
    /// hobby scale this is cheap; if scale grows materially, add a GSI on
    /// `verified_email` and switch to `Query`. (Spec §3.)
    async fn find_identities_by_verified_email(
        &self,
        email: &str,
    ) -> anyhow::Result<Vec<Identity>> {
        // Identity items have pk = IDENTITY#<provider>#<providerId>.  There is
        // no single partition that groups all identities together, so we use a
        // table-wide Scan filtered to items whose pk starts with "IDENTITY#",
        // then post-filter in Rust on the verified_email field stored in `data`.
        let mut results: Vec<Identity> = Vec::new();
        let mut last_evaluated_key = None;

        loop {
            let resp = self
                .client
                .scan()
                .table_name(&self.table)
                .filter_expression("begins_with(pk, :prefix)")
                .expression_attribute_values(":prefix", AttributeValue::S("IDENTITY#".to_owned()))
                .set_exclusive_start_key(last_evaluated_key.clone())
                .send()
                .await
                .context("scan identity partition")?;

            for item in resp.items.unwrap_or_default() {
                let data = item
                    .get("data")
                    .and_then(|v| v.as_s().ok())
                    .context("missing `data` attribute in scan result")?;
                let identity: Identity =
                    serde_json::from_str(data).context("deserialise identity scan item")?;
                if identity.verified_email.as_deref() == Some(email) {
                    results.push(identity);
                }
            }

            last_evaluated_key = resp.last_evaluated_key;
            if last_evaluated_key.is_none() {
                break;
            }
        }

        Ok(results)
    }
}
