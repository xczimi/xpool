//! Signed invite codes (spec §5).
//!
//! Encoding: `urlsafe_b64(serde_json(payload))` + `.` + `urlsafe_b64(hmac_sha256)`.
//! HS256-style with the `INVITE_CODE_SECRET` env var (a 32-byte secret tofu
//! provisions per env). Single-use enforcement (POOL-03 rotation for the
//! multi-use case) is the claim mutation's job — this module only encodes,
//! verifies the signature, and checks expiry.

use base64::Engine;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsePolicy {
    SingleUse,
    MultiUseUntilRotated,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvitePayload {
    pub referrer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub use_policy: UsePolicy,
}

pub fn encode_invite(secret: &[u8], payload: &InvitePayload) -> anyhow::Result<String> {
    let json = serde_json::to_vec(payload)?;
    let body = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&json);
    let mut mac = HmacSha256::new_from_slice(secret)?;
    mac.update(body.as_bytes());
    let sig = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(mac.finalize().into_bytes());
    Ok(format!("{body}.{sig}"))
}

pub fn decode_invite(secret: &[u8], code: &str) -> anyhow::Result<InvitePayload> {
    let (body, sig) = code
        .split_once('.')
        .ok_or_else(|| anyhow::anyhow!("malformed code"))?;
    let mut mac = HmacSha256::new_from_slice(secret)?;
    mac.update(body.as_bytes());
    let actual = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(sig)?;
    // Constant-time compare via the hmac crate's verify_slice (subtle::ConstantTimeEq).
    mac.verify_slice(&actual)
        .map_err(|_| anyhow::anyhow!("signature mismatch"))?;
    let json = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(body)?;
    let payload: InvitePayload = serde_json::from_slice(&json)?;
    if payload.expires_at < Utc::now() {
        anyhow::bail!("expired code");
    }
    Ok(payload)
}
