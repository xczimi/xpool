//! The request clock seam (`.specs/TESTING.md` §3.2).
//!
//! Every request resolves a single `now`, in priority order:
//!   1. the `X-Dev-Now` header   (per-request override — dev/test stub)
//!   2. the `XPOOL_NOW` env var   (process-wide default — dev/test stub)
//!   3. `Utc::now()`              (production)
//!
//! `X-Dev-Now` / `XPOOL_NOW` are dev stubs with the same exposure as
//! `LOCAL_AUTH_ISSUER` — they must be gated off before any real deployment.

use axum::http::HeaderMap;
use chrono::{DateTime, Utc};

/// `now`, placed in the GraphQL context. Resolvers read it from the GraphQL context.
#[derive(Clone, Copy, Debug)]
pub struct RequestNow(pub DateTime<Utc>);

/// Parse an RFC3339 instant; `None` if it does not parse.
fn parse_instant(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s.trim())
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

/// Resolve `now` for a real request: `X-Dev-Now` header, then `XPOOL_NOW`
/// env, then the real clock.
pub fn resolve_now(headers: &HeaderMap) -> DateTime<Utc> {
    let header = headers.get("x-dev-now").and_then(|v| v.to_str().ok());
    let env = std::env::var("XPOOL_NOW").ok();
    resolve_now_from(header, env.as_deref(), Utc::now())
}

/// Resolve `now` from an optional header value and env value. Pure — the
/// real header/env reads happen in the caller, so this is fully testable.
pub fn resolve_now_from(
    header: Option<&str>,
    env: Option<&str>,
    real_now: DateTime<Utc>,
) -> DateTime<Utc> {
    header
        .and_then(parse_instant)
        .or_else(|| env.and_then(parse_instant))
        .unwrap_or(real_now)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn header_wins_over_env_and_real() {
        let got = resolve_now_from(
            Some("2026-06-20T12:00:00Z"),
            Some("2026-07-01T00:00:00Z"),
            t("2026-05-17T00:00:00Z"),
        );
        assert_eq!(got, t("2026-06-20T12:00:00Z"));
    }

    #[test]
    fn env_used_when_no_header() {
        let got = resolve_now_from(
            None,
            Some("2026-07-01T00:00:00Z"),
            t("2026-05-17T00:00:00Z"),
        );
        assert_eq!(got, t("2026-07-01T00:00:00Z"));
    }

    #[test]
    fn real_now_used_when_nothing_set() {
        let real = t("2026-05-17T00:00:00Z");
        assert_eq!(resolve_now_from(None, None, real), real);
    }

    #[test]
    fn unparseable_header_falls_through_to_env() {
        let got = resolve_now_from(
            Some("not-a-date"),
            Some("2026-07-01T00:00:00Z"),
            t("2026-05-17T00:00:00Z"),
        );
        assert_eq!(got, t("2026-07-01T00:00:00Z"));
    }

    #[test]
    fn unparseable_everything_falls_through_to_real() {
        let real = t("2026-05-17T00:00:00Z");
        assert_eq!(resolve_now_from(Some("xxx"), Some("yyy"), real), real);
    }
}
