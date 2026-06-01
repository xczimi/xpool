//! Round-trip a local-issuer JWT — mint with the private key, verify with
//! the public key. The whole multi-issuer seam rests on this primitive.

use api::auth::local_issuer::{mint_for_test, verify_local};

#[test]
fn mint_and_verify_round_trips() {
    let token = mint_for_test("demo-ada", "verified@example.com");
    let claims = verify_local(&token).expect("local-issuer token must verify");
    assert_eq!(claims.sub, "demo-ada");
    assert_eq!(claims.email.as_deref(), Some("verified@example.com"));
    assert_eq!(claims.iss, "xpool-local");
    assert_eq!(claims.aud, "xpool-api");
}

#[test]
fn verify_rejects_a_token_signed_with_the_wrong_key() {
    let result = verify_local("eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJ4In0.bogus");
    assert!(result.is_err());
}
