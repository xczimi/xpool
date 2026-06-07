//! Auth0 JWKS fetcher unit test — mock the HTTP response so the test is
//! offline. Verifies a token signed by a generated keypair whose public
//! half is served as a JWKS.

use api::auth::auth0_jwks::Auth0Verifier;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::json;

#[tokio::test]
async fn verify_accepts_a_token_signed_by_the_jwks_key() {
    // Use the dev-issuer keys as a stand-in; the verifier doesn't care
    // whose JWKS it is, only that the kid matches and the signature
    // verifies.
    let private = include_bytes!("../src/auth/dev_issuer/private_key.pem");

    let header = Header { kid: Some("test-kid".into()), alg: Algorithm::RS256, ..Default::default() };
    let token = encode(
        &header,
        &json!({
            "iss": "https://mock.auth0.test/",
            "aud": "xpool-api",
            "sub": "email|abc",
            "email": "ada@example.com",
            "email_verified": true,
            "exp": (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + 3600) as usize,
        }),
        &EncodingKey::from_rsa_pem(private).unwrap(),
    ).unwrap();

    let jwks: serde_json::Value =
        serde_json::from_str(include_str!("../src/auth/dev_issuer/jwks.json")).unwrap();
    let verifier = Auth0Verifier::with_static_jwks(
        "mock.auth0.test".into(),
        "xpool-api".into(),
        jwks,
    );
    let claims = verifier.verify(&token).await.expect("must verify");
    assert_eq!(claims.sub, "email|abc");
    assert_eq!(claims.verified_email.as_deref(), Some("ada@example.com"));
    assert_eq!(claims.connection, "email");
}

/// Build a signed token with no `email` / `email_verified` — exactly like the
/// real Auth0 access token, which carries only `sub`.
fn token_without_email(sub: &str) -> String {
    let private = include_bytes!("../src/auth/dev_issuer/private_key.pem");
    let header = Header { kid: Some("test-kid".into()), alg: Algorithm::RS256, ..Default::default() };
    encode(
        &header,
        &json!({
            "iss": "https://mock.auth0.test/",
            "aud": "xpool-api",
            "sub": sub,
            "exp": (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + 3600) as usize,
        }),
        &EncodingKey::from_rsa_pem(private).unwrap(),
    )
    .unwrap()
}

fn jwks() -> serde_json::Value {
    serde_json::from_str(include_str!("../src/auth/dev_issuer/jwks.json")).unwrap()
}

/// Auth0 access tokens carry only `sub`; the verifier enriches the verified
/// email from `/userinfo` when it's absent from the token.
#[tokio::test]
async fn verify_enriches_email_from_userinfo() {
    let token = token_without_email("email|6a1d06591a4e901a7b68b616");
    let verifier = Auth0Verifier::with_static_jwks_and_userinfo(
        "mock.auth0.test".into(),
        "xpool-api".into(),
        jwks(),
        json!({ "sub": "email|6a1d06591a4e901a7b68b616", "email": "pool@xczimi.com", "email_verified": true }),
    );
    let claims = verifier.verify(&token).await.expect("must verify");
    assert_eq!(claims.verified_email.as_deref(), Some("pool@xczimi.com"));
    assert_eq!(claims.connection, "email");
}

/// An unverified `/userinfo` email must NOT resolve to a verified email — the
/// caller would otherwise trust an unconfirmed address.
#[tokio::test]
async fn verify_ignores_unverified_userinfo_email() {
    let token = token_without_email("email|deadbeef");
    let verifier = Auth0Verifier::with_static_jwks_and_userinfo(
        "mock.auth0.test".into(),
        "xpool-api".into(),
        jwks(),
        json!({ "sub": "email|deadbeef", "email": "pool@xczimi.com", "email_verified": false }),
    );
    let claims = verifier.verify(&token).await.expect("must verify");
    assert_eq!(claims.verified_email, None);
}
