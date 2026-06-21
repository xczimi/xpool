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

/// A **dev/test-only** stub source driven by the `XPOOL_LIVE_SCORES` env var, so
/// the e2e suite can inject a deterministic live score without touching the
/// network. Format: comma-separated `idEvent=home:away:status`
/// (e.g. `"E1=1:0:2H"`). Inert in production — only constructed when the env var
/// is set (see `build_app`). In the same dev-stub family as `X-Dev-Now`.
pub struct StubLiveSource {
    events: Vec<Event>,
}

impl StubLiveSource {
    /// Parse the `XPOOL_LIVE_SCORES` value. Malformed entries are skipped.
    pub fn parse(spec: &str) -> Self {
        let events = spec
            .split(',')
            .filter_map(|entry| {
                let (id, rest) = entry.split_once('=')?;
                let mut parts = rest.split(':');
                let h: i64 = parts.next()?.trim().parse().ok()?;
                let a: i64 = parts.next()?.trim().parse().ok()?;
                let status = parts.next().unwrap_or("2H").trim().to_string();
                Some(Event {
                    id_event: id.trim().to_string(),
                    date_event: String::new(),
                    id_home_team: String::new(),
                    id_away_team: String::new(),
                    int_home_score: Some(h),
                    int_away_score: Some(a),
                    str_status: status,
                    str_timestamp: None,
                })
            })
            .collect();
        Self { events }
    }

    /// Construct from `XPOOL_LIVE_SCORES`, or `None` when unset/empty.
    pub fn from_env() -> Option<Self> {
        std::env::var("XPOOL_LIVE_SCORES")
            .ok()
            .filter(|s| !s.is_empty())
            .map(|s| Self::parse(&s))
    }
}

#[async_trait]
impl ReportedResultSource for StubLiveSource {
    async fn lookup_events(&self, ids: &[String]) -> anyhow::Result<Vec<Event>> {
        Ok(self
            .events
            .iter()
            .filter(|e| ids.contains(&e.id_event))
            .cloned()
            .collect())
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

    #[tokio::test]
    async fn stub_live_source_parses_env_and_returns_matching_events() {
        // Format: "E1=1:0:2H,E2=3:3:FT" — id=home:away:status, comma-separated.
        let src = StubLiveSource::parse("E1=1:0:2H,E2=3:3:FT");
        let got = src.lookup_events(&["E1".to_string()]).await.unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id_event, "E1");
        assert_eq!(got[0].int_home_score, Some(1));
        assert_eq!(got[0].int_away_score, Some(0));
        assert_eq!(got[0].str_status, "2H");
    }

    #[tokio::test]
    async fn stub_live_source_ignores_unknown_ids() {
        let src = StubLiveSource::parse("E1=1:0:2H");
        let got = src.lookup_events(&["NOPE".to_string()]).await.unwrap();
        assert!(got.is_empty());
    }
}
