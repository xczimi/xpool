//! Multi-issuer JWT verification (spec §2). Trust-list is configured by env:
//! `LOCAL_AUTH_ISSUER` toggles the local issuer; `AUTH0_DOMAIN` +
//! `AUTH0_AUDIENCE` toggle Auth0. Both missing → no Bearer token is
//! accepted (fail closed). Both present is supported (rare, but valid).
//!
//! The verifier returns a `VerifiedClaims` struct — the seam (in
//! resolution.rs) takes it from there.

use crate::auth::auth0_jwks::Auth0Verifier;
use crate::auth::local_issuer::{self, LocalClaims};
use std::env;

/// The trust-list. Built once at startup; cloned cheaply per request.
#[derive(Clone, Default)]
pub struct TrustList {
    pub local: bool,
    pub auth0: Option<Auth0Verifier>,
}

impl TrustList {
    pub fn from_env() -> Self {
        let local = env::var("LOCAL_AUTH_ISSUER")
            .ok()
            .filter(|v| !v.is_empty())
            .is_some();
        let auth0 = match (env::var("AUTH0_DOMAIN").ok(), env::var("AUTH0_AUDIENCE").ok()) {
            (Some(d), Some(a)) if !d.is_empty() && !a.is_empty() => {
                Some(Auth0Verifier::new(d, a))
            }
            _ => None,
        };
        Self { local, auth0 }
    }

    /// Construct a trust-list directly for integration tests. Available when
    /// the `dev_auth` feature is enabled (default) — integration tests in
    /// `tests/` compile as a separate crate and don't see `#[cfg(test)]` items
    /// from the library.
    #[cfg(feature = "dev_auth")]
    pub fn from_env_for_test(local: bool, auth0: Option<Auth0Verifier>) -> Self {
        Self { local, auth0 }
    }

    pub fn is_empty(&self) -> bool {
        !self.local && self.auth0.is_none()
    }
}

/// What the seam consumes downstream — the issuer-neutral verified claims.
#[derive(Clone, Debug)]
pub struct VerifiedClaims {
    pub sub: String,
    pub verified_email: Option<String>,
    pub verified_phone: Option<String>,
    /// "email" | "sms" | "google" | "dev"
    pub connection: String,
}

impl From<LocalClaims> for VerifiedClaims {
    fn from(c: LocalClaims) -> Self {
        Self {
            sub: c.sub,
            verified_email: c.email,
            verified_phone: c.phone_number,
            connection: c.connection.unwrap_or_else(|| "dev".to_owned()),
        }
    }
}

/// Verify a Bearer token against the trust-list. Dispatches by the JWT's
/// `iss` claim (which is unverified at this point — but the matching
/// per-issuer `verify_*` call cryptographically rebinds it).
pub async fn verify_token(
    trust: &TrustList,
    token: &str,
) -> Result<VerifiedClaims, anyhow::Error> {
    if trust.is_empty() {
        anyhow::bail!("no token issuers trusted");
    }
    // Peek at iss without verifying (cheap dispatch).
    let header_iss = peek_iss(token).unwrap_or_default();

    if trust.local && header_iss == local_issuer::ISSUER {
        let claims = local_issuer::verify_local(token)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        return Ok(claims.into());
    }
    if let Some(a) = &trust.auth0 {
        // Auth0 iss is `https://<domain>/`.
        let expected = format!("https://{}/", a.domain);
        if header_iss == expected {
            return a.verify(token).await;
        }
    }
    anyhow::bail!("token issuer not in trust-list: {header_iss}");
}

/// Unverified peek at the JWT payload's `iss` field. Used only for
/// dispatch — the actual verification rebinds the issuer.
fn peek_iss(token: &str) -> Option<String> {
    let mut parts = token.split('.');
    let _header = parts.next()?;
    let payload = parts.next()?;
    let decoded = base64_url_decode(payload)?;
    let v: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    Some(v.get("iss")?.as_str()?.to_owned())
}

fn base64_url_decode(s: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s)
        .ok()
}
