//! The reported-results source seam. The GraphQL resolver depends on this
//! trait (not on `sportsdb` directly) so tests inject a stub and never touch
//! the network. Production wraps the real `sportsdb::SportsDb`; when no API key
//! is configured a `NullSource` returns nothing and result entry degrades to
//! manual (the feature only ever *adds* convenience).

use async_trait::async_trait;
use sportsdb::{Event, SportsDb};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[async_trait]
pub trait ReportedResultSource: Send + Sync {
    /// Recently-finished matches, league-wide. Errors degrade to `Ok(vec![])`
    /// at the call site — see the resolver.
    async fn finished_results(&self) -> anyhow::Result<Vec<Event>>;
}

/// The real source — wraps the SportsDB client.
pub struct SportsDbSource(pub SportsDb);

#[async_trait]
impl ReportedResultSource for SportsDbSource {
    async fn finished_results(&self) -> anyhow::Result<Vec<Event>> {
        self.0.finished_results().await
    }
}

/// Used when `THESPORTSDB_API_KEY` is unset — always empty.
pub struct NullSource;

#[async_trait]
impl ReportedResultSource for NullSource {
    async fn finished_results(&self) -> anyhow::Result<Vec<Event>> {
        Ok(Vec::new())
    }
}

/// A ~45s in-process TTL cache so opening several group screens (or an
/// auto-fetch followed by a manual refresh) doesn't re-hit SportsDB. In-process
/// is enough — Lambda reuses warm containers and the call is cheap regardless.
pub struct CachingSource<S> {
    inner: S,
    ttl: Duration,
    cache: Mutex<Option<(Instant, Vec<Event>)>>,
}

impl<S> CachingSource<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            ttl: Duration::from_secs(45),
            cache: Mutex::new(None),
        }
    }
}

#[async_trait]
impl<S: ReportedResultSource> ReportedResultSource for CachingSource<S> {
    async fn finished_results(&self) -> anyhow::Result<Vec<Event>> {
        if let Some((at, events)) = self.cache.lock().unwrap().as_ref() {
            if at.elapsed() < self.ttl {
                return Ok(events.clone());
            }
        }
        let fresh = self.inner.finished_results().await?;
        *self.cache.lock().unwrap() = Some((Instant::now(), fresh.clone()));
        Ok(fresh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn null_source_is_empty() {
        assert!(NullSource.finished_results().await.unwrap().is_empty());
    }
}
