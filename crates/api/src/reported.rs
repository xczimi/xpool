//! The reported-results source seam. The GraphQL resolver depends on this
//! trait (not on `sportsdb` directly) so tests inject a stub and never touch
//! the network. Production wraps the real `sportsdb::SportsDb`; when no API key
//! is configured a `NullSource` returns nothing and result entry degrades to
//! manual (the feature only ever *adds* convenience).

use async_trait::async_trait;
use sportsdb::{Event, SportsDb};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[async_trait]
pub trait ReportedResultSource: Send + Sync {
    /// Look up the given events individually (accurate status/score). Unknown or
    /// failed ids are simply omitted. `[]` when the source is unconfigured.
    async fn lookup_events(&self, ids: &[String]) -> anyhow::Result<Vec<Event>>;
}

/// The real source — wraps the SportsDB client.
pub struct SportsDbSource(pub SportsDb);

#[async_trait]
impl ReportedResultSource for SportsDbSource {
    async fn lookup_events(&self, ids: &[String]) -> anyhow::Result<Vec<Event>> {
        let mut results = Vec::new();
        for id in ids {
            match self.0.lookup_event(id).await {
                Ok(Some(ev)) => results.push(ev),
                Ok(None) => {}
                Err(_) => {} // per-id error: skip, don't fail the whole batch
            }
        }
        Ok(results)
    }
}

/// Used when `THESPORTSDB_API_KEY` is unset — always empty.
pub struct NullSource;

#[async_trait]
impl ReportedResultSource for NullSource {
    async fn lookup_events(&self, _ids: &[String]) -> anyhow::Result<Vec<Event>> {
        Ok(Vec::new())
    }
}

/// A ~45s in-process TTL cache keyed per event id, so opening several group
/// screens (or an auto-fetch followed by a manual refresh) doesn't re-hit
/// SportsDB. In-process is enough — Lambda reuses warm containers and the
/// call is cheap regardless.
pub struct CachingSource<S> {
    inner: S,
    ttl: Duration,
    cache: Mutex<HashMap<String, (Instant, Event)>>,
}

impl<S> CachingSource<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            ttl: Duration::from_secs(45),
            cache: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl<S: ReportedResultSource> ReportedResultSource for CachingSource<S> {
    async fn lookup_events(&self, ids: &[String]) -> anyhow::Result<Vec<Event>> {
        let now = Instant::now();
        let ttl = self.ttl;

        // Partition: which ids are fresh in cache, which need fetching?
        let (mut cached_events, missing_ids): (Vec<Event>, Vec<String>) = {
            let guard = self.cache.lock().unwrap();
            let mut cached = Vec::new();
            let mut missing = Vec::new();
            for id in ids {
                match guard.get(id) {
                    Some((at, ev)) if now.duration_since(*at) < ttl => {
                        cached.push(ev.clone());
                    }
                    _ => missing.push(id.clone()),
                }
            }
            (cached, missing)
        };

        if !missing_ids.is_empty() {
            let fresh = self.inner.lookup_events(&missing_ids).await?;
            let fetch_time = Instant::now();
            let mut guard = self.cache.lock().unwrap();
            for ev in &fresh {
                guard.insert(ev.id_event.clone(), (fetch_time, ev.clone()));
            }
            cached_events.extend(fresh);
        }

        Ok(cached_events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn null_source_is_empty() {
        assert!(NullSource.lookup_events(&[]).await.unwrap().is_empty());
    }
}
