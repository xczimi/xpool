//! Auth0 JWKS fetcher with an in-memory 1-hour cache.
//!
//! On the first request, fetches `https://{domain}/.well-known/jwks.json`,
//! parses each JWK into a `DecodingKey`, and caches by `kid`. Misses
//! refresh the cache (Auth0 rotates keys occasionally). The cache is
//! per-process — Lambda cold starts re-fetch; warm invocations don't.
//!
//! Auth0 **access tokens** carry only `sub` (no `email` / `email_verified` —
//! those are ID-token / userinfo claims). When the verified email is absent the
//! verifier fetches it from the `/userinfo` endpoint using the same bearer (the
//! token's `aud` includes userinfo), cached per `sub` with the same TTL.

use crate::auth::jwt::VerifiedClaims;
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const CACHE_TTL: Duration = Duration::from_secs(3600);

#[derive(Clone)]
pub struct Auth0Verifier {
    pub domain: String,
    pub audience: String,
    cache: Arc<RwLock<Option<CachedKeys>>>,
    /// Verified email per `sub`, fetched from `/userinfo`. Same TTL as the JWKS.
    userinfo_cache: Arc<RwLock<HashMap<String, CachedUserinfo>>>,
    /// Used by tests to short-circuit the JWKS fetch.
    static_jwks: Option<JwkSet>,
    /// Used by tests to short-circuit the `/userinfo` fetch.
    static_userinfo: Option<serde_json::Value>,
}

struct CachedKeys {
    keys: HashMap<String, DecodingKey>,
    fetched_at: Instant,
}

struct CachedUserinfo {
    verified_email: Option<String>,
    fetched_at: Instant,
}

#[derive(Debug, Deserialize)]
struct Auth0Claims {
    sub: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    email_verified: Option<bool>,
    #[serde(default)]
    phone_number: Option<String>,
    #[serde(default)]
    phone_number_verified: Option<bool>,
}

impl Auth0Verifier {
    pub fn new(domain: String, audience: String) -> Self {
        Self {
            domain,
            audience,
            cache: Arc::new(RwLock::new(None)),
            userinfo_cache: Arc::new(RwLock::new(HashMap::new())),
            static_jwks: None,
            static_userinfo: None,
        }
    }

    /// For tests — bypass the JWKS fetch.
    pub fn with_static_jwks(domain: String, audience: String, jwks: serde_json::Value) -> Self {
        Self {
            static_jwks: Some(serde_json::from_value(jwks).expect("invalid jwks fixture")),
            ..Self::new(domain, audience)
        }
    }

    /// For tests — bypass both the JWKS and `/userinfo` fetches. `userinfo` is
    /// the canned profile JSON the verifier would otherwise GET from Auth0.
    pub fn with_static_jwks_and_userinfo(
        domain: String,
        audience: String,
        jwks: serde_json::Value,
        userinfo: serde_json::Value,
    ) -> Self {
        Self {
            static_userinfo: Some(userinfo),
            ..Self::with_static_jwks(domain, audience, jwks)
        }
    }

    pub async fn verify(&self, token: &str) -> anyhow::Result<VerifiedClaims> {
        let header = decode_header(token)?;
        let kid = header.kid.ok_or_else(|| anyhow::anyhow!("token has no kid"))?;
        let key = self.key_for_kid(&kid).await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[format!("https://{}/", self.domain)]);
        validation.set_audience(&[&self.audience]);
        let data = decode::<Auth0Claims>(token, &key, &validation)?;

        let claims = data.claims;
        let connection = derive_connection(&claims.sub);
        let mut verified_email = claims
            .email_verified
            .unwrap_or(false)
            .then_some(claims.email)
            .flatten();
        // Access tokens omit the email; enrich from `/userinfo` when absent so
        // the email connection (and the link-candidate fallback) can resolve.
        if verified_email.is_none() {
            verified_email = self.userinfo_email(token, &claims.sub).await;
        }
        let verified_phone = claims
            .phone_number_verified
            .unwrap_or(false)
            .then_some(claims.phone_number)
            .flatten();

        Ok(VerifiedClaims {
            sub: claims.sub,
            verified_email,
            verified_phone,
            connection,
        })
    }

    /// Verified email from Auth0 `/userinfo`, cached per `sub`. Returns `None`
    /// on any failure or if the email is unverified — the caller then resolves
    /// to `AuthenticatedUnclaimed`/`Visitor`, never a wrong identity.
    async fn userinfo_email(&self, token: &str, sub: &str) -> Option<String> {
        {
            let guard = self.userinfo_cache.read().await;
            if let Some(cached) = guard.get(sub) {
                if cached.fetched_at.elapsed() < CACHE_TTL {
                    return cached.verified_email.clone();
                }
            }
        }
        let verified_email = self
            .fetch_userinfo(token)
            .await
            .as_ref()
            .and_then(verified_email_from_userinfo);
        self.userinfo_cache.write().await.insert(
            sub.to_owned(),
            CachedUserinfo {
                verified_email: verified_email.clone(),
                fetched_at: Instant::now(),
            },
        );
        verified_email
    }

    async fn fetch_userinfo(&self, token: &str) -> Option<serde_json::Value> {
        if let Some(value) = &self.static_userinfo {
            return Some(value.clone());
        }
        let url = format!("https://{}/userinfo", self.domain);
        match reqwest::Client::new().get(&url).bearer_auth(token).send().await {
            Ok(resp) => resp.json::<serde_json::Value>().await.ok(),
            Err(e) => {
                tracing::warn!("userinfo fetch failed: {e}");
                None
            }
        }
    }

    async fn key_for_kid(&self, kid: &str) -> anyhow::Result<DecodingKey> {
        if let Some(jwks) = &self.static_jwks {
            return jwk_to_decoding(jwks, kid);
        }
        // Fast path: read lock, check cache.
        {
            let guard = self.cache.read().await;
            if let Some(cached) = guard.as_ref() {
                if cached.fetched_at.elapsed() < CACHE_TTL {
                    if let Some(k) = cached.keys.get(kid) {
                        return Ok(k.clone());
                    }
                }
            }
        }
        // Slow path: fetch + cache.
        let jwks = self.fetch_jwks().await?;
        let key = jwk_to_decoding(&jwks, kid)?;
        let mut map = HashMap::new();
        for jwk in jwks.keys.iter() {
            if let Some(k) = jwk.common.key_id.clone() {
                if let Ok(dk) = DecodingKey::from_jwk(jwk) {
                    map.insert(k, dk);
                }
            }
        }
        *self.cache.write().await = Some(CachedKeys {
            keys: map,
            fetched_at: Instant::now(),
        });
        Ok(key)
    }

    async fn fetch_jwks(&self) -> anyhow::Result<JwkSet> {
        let url = format!("https://{}/.well-known/jwks.json", self.domain);
        let resp = reqwest::Client::new().get(&url).send().await?;
        Ok(resp.json::<JwkSet>().await?)
    }
}

fn jwk_to_decoding(jwks: &JwkSet, kid: &str) -> anyhow::Result<DecodingKey> {
    let jwk = jwks
        .find(kid)
        .ok_or_else(|| anyhow::anyhow!("kid {kid} not in jwks"))?;
    Ok(DecodingKey::from_jwk(jwk)?)
}

/// Extract a *verified* email from an Auth0 `/userinfo` profile. Returns `None`
/// unless `email_verified` is true and `email` is present.
fn verified_email_from_userinfo(value: &serde_json::Value) -> Option<String> {
    if value.get("email_verified").and_then(|v| v.as_bool()) != Some(true) {
        return None;
    }
    value
        .get("email")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
}

/// Derive a connection name from Auth0's `sub` prefix. Auth0 sub formats:
///   - `email|xxx`           → passwordless email
///   - `sms|xxx`             → passwordless SMS
///   - `google-oauth2|xxx`   → Google social
fn derive_connection(sub: &str) -> String {
    match sub.split_once('|').map(|(p, _)| p) {
        Some("email") => "email".to_owned(),
        Some("sms") => "sms".to_owned(),
        Some("google-oauth2") => "google".to_owned(),
        _ => "unknown".to_owned(),
    }
}
