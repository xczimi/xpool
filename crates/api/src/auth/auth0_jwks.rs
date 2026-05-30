//! Auth0 JWKS fetcher with an in-memory 1-hour cache.
//!
//! On the first request, fetches `https://{domain}/.well-known/jwks.json`,
//! parses each JWK into a `DecodingKey`, and caches by `kid`. Misses
//! refresh the cache (Auth0 rotates keys occasionally). The cache is
//! per-process — Lambda cold starts re-fetch; warm invocations don't.

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
    /// Used by tests to short-circuit the HTTP fetch.
    static_jwks: Option<JwkSet>,
}

struct CachedKeys {
    keys: HashMap<String, DecodingKey>,
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
            static_jwks: None,
        }
    }

    /// For tests — bypass the JWKS fetch.
    pub fn with_static_jwks(domain: String, audience: String, jwks: serde_json::Value) -> Self {
        Self {
            domain,
            audience,
            cache: Arc::new(RwLock::new(None)),
            static_jwks: Some(serde_json::from_value(jwks).expect("invalid jwks fixture")),
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
        let verified_email = claims
            .email_verified
            .unwrap_or(false)
            .then_some(claims.email)
            .flatten();
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
