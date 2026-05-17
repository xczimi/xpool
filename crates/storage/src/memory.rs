//! In-memory `Repository` fake for unit/integration tests (task P3).
//!
//! `InMemoryRepository` is cheap to clone: all clones share the same inner
//! `Arc<Mutex<...>>` state, so tests can hold multiple handles to one store.

use crate::{Repository, Scoreboard};
use async_trait::async_trait;
use domain::{Identity, Person, Player, PlayerId, Pool, Tournament};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// Shared inner state, wrapped in `Arc<Mutex<...>>`.
#[derive(Default)]
struct Inner {
    tournament: Option<Tournament>,
    players: HashMap<PlayerId, Player>,
    scoreboard: Option<Scoreboard>,
    pools: HashMap<String, Pool>,
    /// Keyed `"<provider>#<provider_id>"` → `Identity`.
    identities: HashMap<String, Identity>,
    persons: HashMap<String, Person>,
}

/// Mutex-wrapped in-memory store. Cheap to clone (all clones share the inner
/// state via `Arc`).
#[derive(Clone, Default)]
pub struct InMemoryRepository {
    inner: Arc<Mutex<Inner>>,
}

impl InMemoryRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Repository for InMemoryRepository {
    // ── Tournament ─────────────────────────────────────────────────────────

    async fn get_tournament(&self) -> anyhow::Result<Option<Tournament>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.tournament.clone())
    }

    async fn put_tournament(&self, t: &Tournament) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.tournament = Some(t.clone());
        Ok(())
    }

    // ── Player ─────────────────────────────────────────────────────────────

    async fn get_player(&self, id: &str) -> anyhow::Result<Option<Player>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.players.get(id).cloned())
    }

    async fn list_players(&self) -> anyhow::Result<Vec<Player>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.players.values().cloned().collect())
    }

    /// Optimistic concurrency: if a player with `p.id` already exists and its
    /// stored `version` differs from `p.version`, returns `Err`. On success
    /// stores the player as given (the caller is responsible for bumping
    /// `version` before calling this).
    async fn put_player(&self, p: &Player) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(existing) = inner.players.get(&p.id) {
            if existing.version != p.version {
                anyhow::bail!(
                    "optimistic concurrency conflict for player {}: \
                     stored version {} != supplied version {}",
                    p.id,
                    existing.version,
                    p.version,
                );
            }
        }
        inner.players.insert(p.id.clone(), p.clone());
        Ok(())
    }

    // ── Scoreboard ─────────────────────────────────────────────────────────

    async fn get_scoreboard(&self) -> anyhow::Result<Option<Scoreboard>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.scoreboard.clone())
    }

    async fn put_scoreboard(&self, s: &Scoreboard) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.scoreboard = Some(s.clone());
        Ok(())
    }

    // ── Pool ───────────────────────────────────────────────────────────────

    async fn list_pools(&self) -> anyhow::Result<Vec<Pool>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.pools.values().cloned().collect())
    }

    async fn put_pool(&self, p: &Pool) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.pools.insert(p.id.clone(), p.clone());
        Ok(())
    }

    // ── Identity ───────────────────────────────────────────────────────────

    async fn get_identity(
        &self,
        provider: &str,
        provider_id: &str,
    ) -> anyhow::Result<Option<Identity>> {
        let key = format!("{provider}#{provider_id}");
        let inner = self.inner.lock().unwrap();
        Ok(inner.identities.get(&key).cloned())
    }

    async fn put_identity(&self, i: &Identity) -> anyhow::Result<()> {
        let key = format!("{}#{}", i.provider, i.provider_id);
        let mut inner = self.inner.lock().unwrap();
        inner.identities.insert(key, i.clone());
        Ok(())
    }

    // ── Person ─────────────────────────────────────────────────────────────

    async fn get_person(&self, id: &str) -> anyhow::Result<Option<Person>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.persons.get(id).cloned())
    }

    async fn put_person(&self, p: &Person) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.persons.insert(p.id.clone(), p.clone());
        Ok(())
    }
}
