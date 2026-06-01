use api::auth::invite_code::{decode_invite, encode_invite, InvitePayload, UsePolicy};
use chrono::{Duration, Utc};

const SECRET: &str = "test-only-secret-32-bytes-long-xx";

#[test]
fn round_trip_a_referral_code() {
    let payload = InvitePayload {
        referrer: "demo-ada".into(),
        pool: None,
        expires_at: Utc::now() + Duration::days(14),
        use_policy: UsePolicy::SingleUse,
    };
    let encoded = encode_invite(SECRET.as_bytes(), &payload).unwrap();
    let decoded = decode_invite(SECRET.as_bytes(), &encoded).unwrap();
    assert_eq!(decoded.referrer, "demo-ada");
    assert!(decoded.pool.is_none());
    assert!(matches!(decoded.use_policy, UsePolicy::SingleUse));
}

#[test]
fn round_trip_a_pool_join_code() {
    let payload = InvitePayload {
        referrer: "demo-ada".into(),
        pool: Some("pool-xyz".into()),
        expires_at: Utc::now() + Duration::days(30),
        use_policy: UsePolicy::MultiUseUntilRotated,
    };
    let encoded = encode_invite(SECRET.as_bytes(), &payload).unwrap();
    let decoded = decode_invite(SECRET.as_bytes(), &encoded).unwrap();
    assert_eq!(decoded.pool.as_deref(), Some("pool-xyz"));
    assert!(matches!(decoded.use_policy, UsePolicy::MultiUseUntilRotated));
}

#[test]
fn decode_rejects_a_tampered_code() {
    let payload = InvitePayload {
        referrer: "demo-ada".into(),
        pool: None,
        expires_at: Utc::now() + Duration::days(14),
        use_policy: UsePolicy::SingleUse,
    };
    let mut encoded = encode_invite(SECRET.as_bytes(), &payload).unwrap();
    encoded.push('a');
    assert!(decode_invite(SECRET.as_bytes(), &encoded).is_err());
}

#[test]
fn decode_rejects_an_expired_code() {
    let payload = InvitePayload {
        referrer: "demo-ada".into(),
        pool: None,
        expires_at: Utc::now() - Duration::days(1),
        use_policy: UsePolicy::SingleUse,
    };
    let encoded = encode_invite(SECRET.as_bytes(), &payload).unwrap();
    assert!(decode_invite(SECRET.as_bytes(), &encoded).is_err());
}

#[test]
fn decode_rejects_wrong_secret() {
    let payload = InvitePayload {
        referrer: "demo-ada".into(),
        pool: None,
        expires_at: Utc::now() + Duration::days(14),
        use_policy: UsePolicy::SingleUse,
    };
    let encoded = encode_invite(SECRET.as_bytes(), &payload).unwrap();
    assert!(decode_invite(b"some-other-secret-also-32-bytes-x", &encoded).is_err());
}
