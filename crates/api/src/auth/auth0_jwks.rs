//! Auth0 JWKS fetcher with an in-memory 1-hour cache. The real
//! implementation lands in Task 6; this stub keeps the module graph valid.

use jsonwebtoken::DecodingKey;

#[derive(Clone)]
pub struct Auth0Verifier {
    pub domain: String,
    pub audience: String,
}

impl Auth0Verifier {
    pub fn new(domain: String, audience: String) -> Self {
        Self { domain, audience }
    }

    pub async fn verify(
        &self,
        _token: &str,
    ) -> Result<crate::auth::jwt::VerifiedClaims, anyhow::Error> {
        anyhow::bail!("auth0 verifier not yet implemented (Task 6)")
    }

    #[allow(dead_code)]
    fn _decoding_key_placeholder(&self) -> Option<DecodingKey> {
        None
    }
}
