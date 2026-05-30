//! The local-issuer RS256 path. Loads the committed test keypair, exposes
//! `mint_for_test` (used by the dev-login endpoint and unit tests) and
//! `verify_local` (used by the multi-issuer verifier when
//! `LOCAL_AUTH_ISSUER` is set).
//!
//! Issuer / audience are constants: production binaries built with
//! `--features lambda` do not include this module, but defence-in-depth
//! also requires the runtime trust-list to opt in via env (see jwt.rs).

use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};

pub const ISSUER: &str = "xpool-local";
pub const AUDIENCE: &str = "xpool-api";

const PRIVATE_PEM: &[u8] = include_bytes!("dev_issuer/private_key.pem");
const PUBLIC_PEM: &[u8] = include_bytes!("dev_issuer/public_key.pem");

/// Verified claims shape — a subset that matches what Auth0 also emits, so
/// the seam treats both issuers uniformly.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<String>,
    /// The Auth0 connection name we mimic ("email" / "sms" / "google").
    /// For dev-login this is always "dev".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
}

fn encoding_key() -> EncodingKey {
    EncodingKey::from_rsa_pem(PRIVATE_PEM).expect("dev_issuer/private_key.pem is invalid")
}

fn decoding_key() -> DecodingKey {
    DecodingKey::from_rsa_pem(PUBLIC_PEM).expect("dev_issuer/public_key.pem is invalid")
}

/// Mint a token for tests / dev-login. 1-hour TTL.
pub fn mint_for_test(sub: &str, email: &str) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    let claims = LocalClaims {
        sub: sub.to_owned(),
        iss: ISSUER.to_owned(),
        aud: AUDIENCE.to_owned(),
        exp: now + 3600,
        email: Some(email.to_owned()),
        phone_number: None,
        connection: Some("dev".to_owned()),
    };
    encode(&Header::new(Algorithm::RS256), &claims, &encoding_key())
        .expect("encoding a local-issuer JWT must not fail")
}

/// Verify a token against the local issuer. Errors when the signature,
/// issuer, audience, or expiry don't check out.
pub fn verify_local(token: &str) -> Result<LocalClaims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[ISSUER]);
    validation.set_audience(&[AUDIENCE]);
    let data = decode::<LocalClaims>(token, &decoding_key(), &validation)?;
    Ok(data.claims)
}
