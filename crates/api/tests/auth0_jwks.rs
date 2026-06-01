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
