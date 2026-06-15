//! HTTP client for TheSportsDB V2 (header auth). League/season are fixed to
//! FIFA World Cup 2026 (`idLeague 4429`, season `2026`,
//! `.specs/THESPORTSDB_API.md` §3). Always pulls *league-wide* endpoints in one
//! call and lets callers filter locally — never loops per-event (§2 rate limits).

use crate::decode::decode_schedule;
use crate::model::Event;
use std::time::Duration;

const BASE: &str = "https://www.thesportsdb.com/api/v2/json";
const LEAGUE_ID: &str = "4429";
const SEASON: &str = "2026";

pub struct SportsDb {
    http: reqwest::Client,
    key: String,
}

impl SportsDb {
    /// Build from `THESPORTSDB_API_KEY`. Returns `None` when unset/empty so
    /// every consumer can degrade gracefully to "no reported results".
    pub fn from_env() -> Option<Self> {
        let key = std::env::var("THESPORTSDB_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())?;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .ok()?;
        Some(Self { http, key })
    }

    /// Full season schedule (every fixture) — the reconcile source.
    pub async fn season_schedule(&self) -> anyhow::Result<Vec<Event>> {
        let url = format!("{BASE}/schedule/league/{LEAGUE_ID}/{SEASON}");
        decode_schedule(&self.get(&url).await?)
    }

    /// Recently-finished matches — the result-entry source. One league-wide call.
    pub async fn finished_results(&self) -> anyhow::Result<Vec<Event>> {
        let url = format!("{BASE}/schedule/previous/league/{LEAGUE_ID}");
        decode_schedule(&self.get(&url).await?)
    }

    /// GET with one retry. Errors are returned, never panicked.
    async fn get(&self, url: &str) -> anyhow::Result<String> {
        let mut last_err = None;
        for _ in 0..2 {
            match self
                .http
                .get(url)
                .header("X-API-KEY", &self.key)
                .send()
                .await
            {
                Ok(resp) => return Ok(resp.error_for_status()?.text().await?),
                Err(e) => last_err = Some(e),
            }
        }
        Err(anyhow::anyhow!("sportsdb GET failed: {:?}", last_err))
    }
}
