# xpool Auth Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the `X-Dev-Player` dev stub with real, production-ready Auth0-brokered passwordless authentication, fronted by an in-app Bearer-JWT seam that also accepts a local-issuer token for dev and tests. Implement the shareable invite-link claim flow that AUTH-07/AUTH-11 collapse into.

**Architecture:** Two-layer axum middleware (`cloudfront_auth` → JWT seam) on the API; multi-issuer JWT verification (Auth0 JWKS + local RS256 keypair) gated by env vars (`AUTH0_DOMAIN`, `LOCAL_AUTH_ISSUER`); three-state `CurrentPlayer` (`Visitor` / `AuthenticatedUnclaimed` / `Player`); lazy `Person`/`Player` creation at claim time, driven by signed invite codes. Frontend uses `@auth0/auth0-react` in production and a dev-login endpoint locally — both emit Bearer JWTs the API verifies the same way.

**Tech Stack:** Rust (`axum`, `async-graphql`, `jsonwebtoken` v9, `reqwest` for JWKS), React + Vite (`@auth0/auth0-react`), DynamoDB (existing single-table), Playwright (e2e), Auth0 (passwordless email via SES + passwordless SMS via Twilio + Google).

**Source spec:** `docs/superpowers/specs/2026-05-30-auth-design.md`.

---

## File Structure

### Backend — `crates/api/src/auth/` (new module, replaces `auth.rs`)

| File | Responsibility |
|---|---|
| `mod.rs` | Re-exports; `CurrentPlayer` enum (moved here from `auth.rs`). |
| `seam.rs` | The axum middleware: `Authorization: Bearer <jwt>` → `CurrentPlayer` in request extensions. |
| `jwt.rs` | Multi-issuer verification: trusted-issuer registry, `verify_token()`. |
| `auth0_jwks.rs` | JWKS fetch + 1h cache for the `AUTH0_DOMAIN` issuer. |
| `local_issuer.rs` | Static RS256 public key load for the local issuer. Used by both verification and tests. |
| `resolution.rs` | `verified_claims → Identity → Person → Player` algorithm (§3 of the spec). |
| `dev_login.rs` | `POST /api/dev/login` endpoint. Mints local-issuer JWTs. Compiled out of `--features lambda` builds. |
| `invite_code.rs` | HS256-signed invite-code payload (referrer + optional pool + expiry + use-policy). |
| `dev_issuer/private_key.pem` | Test-only RS256 private key (committed; useless without `LOCAL_AUTH_ISSUER` env). |
| `dev_issuer/public_key.pem` | Matching public key. |

### Backend — `crates/api/src/router.rs` (modify)

- Drop `resolve_current_player` / `x-dev-player` header read.
- Mount `auth::seam` middleware before the GraphQL handler.
- Mount `/api/dev/login` route (feature-gated).

### Backend — `crates/api/src/gql/` (modify)

- `me.rs`: handle `CurrentPlayer::AuthenticatedUnclaimed` (return an `UnclaimedViewer` GraphQL type).
- New mutations: `createInvite`, `claimInvite`, `confirmLink`.

### Backend — `crates/domain/src/model.rs` (modify, locked-contract change)

- Add `Identity.verified_email: Option<String>`.

### Backend — `crates/storage/src/lib.rs` + adapters (modify)

- Add `Repository::find_identities_by_verified_email(email) -> Vec<Identity>`.
- Implementations: `InMemoryRepository` scans; `DynamoRepository` scans the global zone (acceptable for hobby scale; flagged in code comments).

### Frontend — `web/src/auth/` (modify)

| File | Change |
|---|---|
| `devAuth.ts` | Replace player-id storage with JWT storage; add `devLogin(playerId)` API call. |
| `AuthContext.tsx` | Three states: `visitor` / `unclaimed` / `player`. Holds JWT + claims summary. |
| `authContextValue.ts` | Updated `AuthState` type. |
| `auth0Provider.tsx` | New — wraps `@auth0/auth0-react`, mounted only when `VITE_AUTH0_DOMAIN` is set. |

### Frontend — `web/src/graphql/client.ts` (modify)

- Send `Authorization: Bearer <jwt>` instead of `X-Dev-Player`.

### Frontend — `web/src/pages/` (modify + new)

| File | Change |
|---|---|
| `InvitePage.tsx` | Show generated link (and optional "also email" form). |
| `InviteClaimPage.tsx` | New — `/invite/:code` landing; "log in to claim" → calls `claimInvite`. |
| `LinkConfirmPage.tsx` | New — AUTH-13 explicit-confirmation prompt. |

### Frontend — `web/src/components/AuthBar.tsx` (modify)

- In dev mode (no `VITE_AUTH0_DOMAIN`): player picker → calls `POST /api/dev/login`.
- In prod mode: "Log in" button → `loginWithRedirect()`.

### e2e — `web/e2e/` (modify)

- `helpers.ts`: `devLogin()` calls the dev-login endpoint and stashes the JWT.
- `auth.spec.ts`: assert the unclaimed state; add invite-link end-to-end test.
- `scripts/e2e-stack.sh`: export `LOCAL_AUTH_ISSUER=1`.

### Specs — `.specs/` (modify)

- `DATA_MODEL.md` §12, §3.
- `API.md` §8.
- `SCENARIOS.md` decision bullet + AUTH-01/02/05/07/08/09/10/11 + new AUTH-18.

---

## Phase 1 — Backend JWT seam (replaces the X-Dev-Player path)

### Task 1: Add JWT dependencies

**Files:**
- Modify: `crates/api/Cargo.toml`

- [ ] **Step 1: Add the deps**

Add to `[dependencies]`:

```toml
jsonwebtoken = "9"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
```

Add to `[features]` (next to the existing `default = []` and `lambda = [...]`):

```toml
# Compiled in for `cargo build` / `cargo run` (no feature flags) and the
# default `cargo test`. Disabled in `--features lambda` production builds.
# Gates the dev-login endpoint and `LOCAL_AUTH_ISSUER` env-var handling.
dev_auth = []
default = ["dev_auth"]
```

Update the `lambda` feature line to be exclusive — leave `default = ["dev_auth"]` alone and document at the top of `bin/deploy-api` that production uses `cargo build --release --no-default-features --features lambda`.

- [ ] **Step 2: Verify it compiles**

```bash
cargo check -p api
cargo check -p api --no-default-features --features lambda
```

Expected: both clean.

- [ ] **Step 3: Commit**

```bash
git add crates/api/Cargo.toml
git commit -m "feat(api): jsonwebtoken + reqwest deps; dev_auth feature gate"
```

---

### Task 2: Generate the local-issuer keypair fixture

**Files:**
- Create: `crates/api/src/auth/dev_issuer/private_key.pem`
- Create: `crates/api/src/auth/dev_issuer/public_key.pem`
- Create: `crates/api/src/auth/dev_issuer/README.md`

- [ ] **Step 1: Generate the keypair**

```bash
mkdir -p crates/api/src/auth/dev_issuer
openssl genpkey -algorithm RSA -out crates/api/src/auth/dev_issuer/private_key.pem -pkeyopt rsa_keygen_bits:2048
openssl rsa -in crates/api/src/auth/dev_issuer/private_key.pem -pubout -out crates/api/src/auth/dev_issuer/public_key.pem
```

- [ ] **Step 2: Write the README**

```markdown
# Dev-issuer keypair

Test-only RS256 keypair for the `LOCAL_AUTH_ISSUER` JWT path
(see `docs/superpowers/specs/2026-05-30-auth-design.md` §2).

The private key has **no power** unless `LOCAL_AUTH_ISSUER` is set in the
environment, which never happens in production (the env var is unset in
`infrastructure/lambda.tf`, and the `dev_login` module that uses the key
is gated off the `dev_auth` Cargo feature, which is excluded from
`--features lambda` builds).

Regenerate: see the commands in `docs/superpowers/plans/2026-05-30-xpool-auth.md`
Task 2.
```

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/auth/dev_issuer/
git commit -m "feat(api): dev-issuer RS256 keypair fixture"
```

---

### Task 3: Local-issuer module — load + sign + verify against the fixture keypair

**Files:**
- Create: `crates/api/src/auth/local_issuer.rs`
- Modify: `crates/api/src/auth/mod.rs` (create — replaces `auth.rs` in Task 4)

- [ ] **Step 1: Write the failing test**

Create `crates/api/tests/local_issuer.rs`:

```rust
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
    // Hand-craft a token with the same header but a junk signature.
    let parts: Vec<&str> = "eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJ4In0.bogus".split('.').collect();
    assert_eq!(parts.len(), 3);
    let result = verify_local("eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJ4In0.bogus");
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run, see it fail**

```bash
cargo test -p api --test local_issuer 2>&1 | head -20
```

Expected: `error[E0432]: unresolved import \`api::auth::local_issuer\``.

- [ ] **Step 3: Replace `auth.rs` with the new `auth/` module skeleton**

Delete `crates/api/src/auth.rs` (its body is moved into Task 4). Create `crates/api/src/auth/mod.rs`:

```rust
//! The auth seam (`docs/superpowers/specs/2026-05-30-auth-design.md`).
//!
//! Bearer-JWT verification, multi-issuer (Auth0 + local). Three-state
//! `CurrentPlayer`. Identity → Person → Player resolution. The
//! `X-Dev-Player` header is gone — local dev mints local-issuer JWTs via
//! the dev-login endpoint instead (one auth code path).

pub mod local_issuer;

// Filled in by later tasks:
//   pub mod jwt;
//   pub mod auth0_jwks;
//   pub mod resolution;
//   pub mod seam;
//   pub mod invite_code;
//   #[cfg(feature = "dev_auth")] pub mod dev_login;

// CurrentPlayer moves here in Task 4. Re-exported at the module root for
// callers that used `crate::auth::CurrentPlayer` previously.
```

- [ ] **Step 4: Write `local_issuer.rs`**

```rust
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
```

- [ ] **Step 5: Run the test — it should pass**

```bash
cargo test -p api --test local_issuer
```

Expected: both tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/auth.rs crates/api/src/auth/ crates/api/tests/local_issuer.rs
git commit -m "feat(api): local-issuer RS256 mint+verify"
```

(`auth.rs` deletion is part of the same change — the new `auth/mod.rs` replaces it.)

---

### Task 4: Three-state `CurrentPlayer` + `require` / `require_admin`

**Files:**
- Modify: `crates/api/src/auth/mod.rs`
- Modify: every caller of `CurrentPlayer::Authenticated` — find with `rg`.

- [ ] **Step 1: Write the failing test**

Append to `crates/api/tests/local_issuer.rs` (or create `crates/api/tests/current_player.rs`):

```rust
use api::auth::{CurrentPlayer, VerifiedIdentity};
use async_graphql::{Context, EmptyMutation, EmptySubscription, Object, Schema};
use domain::Player;

struct Q;
#[Object]
impl Q {
    async fn require_player_id(&self, ctx: &Context<'_>) -> async_graphql::Result<String> {
        Ok(CurrentPlayer::require(ctx)?.id.clone())
    }
}

#[tokio::test]
async fn unclaimed_is_not_a_player() {
    let schema = Schema::new(Q, EmptyMutation, EmptySubscription);
    let viewer = CurrentPlayer::AuthenticatedUnclaimed(VerifiedIdentity {
        connection: "email".into(),
        sub: "auth0|abc".into(),
        verified_email: Some("x@example.com".into()),
        verified_phone: None,
    });
    let res = schema
        .execute(async_graphql::Request::new("{ requirePlayerId }").data(viewer))
        .await;
    assert!(!res.errors.is_empty(), "unclaimed must fail require()");
}
```

- [ ] **Step 2: Run, see it fail**

```bash
cargo test -p api --test local_issuer unclaimed_is_not_a_player 2>&1 | head -20
```

Expected: `error[E0432]: unresolved import \`api::auth::VerifiedIdentity\``.

- [ ] **Step 3: Implement the enum**

Append to `crates/api/src/auth/mod.rs`:

```rust
use async_graphql::Context;
use domain::Player;

/// The verified-identity claims-set the seam extracts from a JWT, *before*
/// any Identity/Person lookup. Carried through the AUTH-06 unclaimed state
/// so the claim flow can act on it.
#[derive(Clone, Debug)]
pub struct VerifiedIdentity {
    /// "email" | "sms" | "google" | "dev"
    pub connection: String,
    /// The original `sub` from the JWT (Auth0 connection-specific or local).
    pub sub: String,
    pub verified_email: Option<String>,
    pub verified_phone: Option<String>,
}

/// The viewer of a request, placed in the GraphQL context.
#[derive(Clone, Debug)]
pub enum CurrentPlayer {
    /// No / invalid token.
    Visitor,
    /// Valid token, verified contact, but no `Person`/`Player` (AUTH-06).
    AuthenticatedUnclaimed(VerifiedIdentity),
    /// Resolved `Player` (including the result-user).
    Player(Box<Player>),
}

impl CurrentPlayer {
    /// The authenticated player, or a GraphQL auth error otherwise.
    pub fn require<'a>(ctx: &'a Context<'_>) -> async_graphql::Result<&'a Player> {
        match ctx.data_unchecked::<CurrentPlayer>() {
            CurrentPlayer::Player(p) => Ok(p),
            CurrentPlayer::AuthenticatedUnclaimed(_) => {
                Err(async_graphql::Error::new("invitation required"))
            }
            CurrentPlayer::Visitor => {
                Err(async_graphql::Error::new("authentication required"))
            }
        }
    }

    pub fn require_admin<'a>(ctx: &'a Context<'_>) -> async_graphql::Result<&'a Player> {
        let player = Self::require(ctx)?;
        if !player.is_result_user {
            return Err(async_graphql::Error::new(
                "admin privileges required (result user only)",
            ));
        }
        Ok(player)
    }
}
```

- [ ] **Step 4: Migrate every `Authenticated(...)` call site**

```bash
rg -n "CurrentPlayer::Authenticated" crates/ web/
```

Replace each `CurrentPlayer::Authenticated(p)` with `CurrentPlayer::Player(p)`. Expected callers: resolvers under `crates/api/src/gql/`, tests under `crates/api/tests/`. Run after each: `cargo check -p api` to catch misses.

- [ ] **Step 5: Run all tests**

```bash
cargo test -p api 2>&1 | tail -30
```

Expected: the new test passes; the migrated callers compile. Pre-existing seam tests may fail because the header path is still in place — fix in Task 5; for now, target the new test.

```bash
cargo test -p api --test local_issuer
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/auth/ crates/api/src/gql/ crates/api/tests/
git commit -m "feat(api): three-state CurrentPlayer (Visitor / Unclaimed / Player)"
```

---

### Task 5: Multi-issuer verifier — trust-list from env, dispatch by `iss`

**Files:**
- Create: `crates/api/src/auth/jwt.rs`
- Create: `crates/api/src/auth/auth0_jwks.rs` (stub for now; implemented in Task 6)
- Modify: `crates/api/src/auth/mod.rs` (re-export)

- [ ] **Step 1: Write the failing tests**

Create `crates/api/tests/jwt_verify.rs`:

```rust
//! The multi-issuer verifier dispatches by the trust-list configured in
//! env vars. Auth0 path is stubbed in this test (no network); the local
//! path runs end-to-end.

use api::auth::jwt::{verify_token, TrustList};
use api::auth::local_issuer::mint_for_test;

#[tokio::test]
async fn local_issuer_only_accepts_local_tokens() {
    let trust = TrustList::from_env_for_test(/* local */ true, /* auth0 */ None);
    let token = mint_for_test("demo-ada", "ada@example.com");
    let verified = verify_token(&trust, &token).await.expect("local must verify");
    assert_eq!(verified.sub, "demo-ada");
    assert_eq!(verified.verified_email.as_deref(), Some("ada@example.com"));
}

#[tokio::test]
async fn empty_trustlist_rejects_everything() {
    let trust = TrustList::from_env_for_test(false, None);
    let token = mint_for_test("demo-ada", "ada@example.com");
    assert!(verify_token(&trust, &token).await.is_err());
}

#[tokio::test]
async fn malformed_token_fails_closed() {
    let trust = TrustList::from_env_for_test(true, None);
    assert!(verify_token(&trust, "not.a.jwt").await.is_err());
}
```

- [ ] **Step 2: Run, see it fail**

```bash
cargo test -p api --test jwt_verify 2>&1 | head -20
```

Expected: unresolved import.

- [ ] **Step 3: Stub the Auth0 JWKS module**

Create `crates/api/src/auth/auth0_jwks.rs`:

```rust
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
```

- [ ] **Step 4: Implement `jwt.rs`**

```rust
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

    #[cfg(test)]
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
        let claims = local_issuer::verify_local(token)?;
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
```

Add `base64 = "0.22"` to `crates/api/Cargo.toml` dependencies.

- [ ] **Step 5: Wire the module in `auth/mod.rs`**

```rust
pub mod jwt;
pub mod auth0_jwks;
```

- [ ] **Step 6: Run the tests**

```bash
cargo test -p api --test jwt_verify
```

Expected: all three pass.

- [ ] **Step 7: Commit**

```bash
git add crates/api/Cargo.toml crates/api/src/auth/ crates/api/tests/jwt_verify.rs
git commit -m "feat(api): multi-issuer JWT verifier with env-built trust-list"
```

---

### Task 6: Auth0 JWKS fetcher with 1-hour cache

**Files:**
- Modify: `crates/api/src/auth/auth0_jwks.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/api/tests/auth0_jwks.rs`:

```rust
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
    let public = include_bytes!("../src/auth/dev_issuer/public_key.pem");

    let header = Header { kid: Some("test-kid".into()), alg: Algorithm::RS256, ..Default::default() };
    let token = encode(
        &header,
        &json!({
            "iss": "https://mock.auth0.test/",
            "aud": "xpool-api",
            "sub": "auth0|abc",
            "email": "ada@example.com",
            "email_verified": true,
            "exp": (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + 3600) as usize,
        }),
        &EncodingKey::from_rsa_pem(private).unwrap(),
    ).unwrap();

    // Inject a stubbed JWKS.
    let jwks = mock_jwks_from_pem("test-kid", public);
    let verifier = Auth0Verifier::with_static_jwks(
        "mock.auth0.test".into(),
        "xpool-api".into(),
        jwks,
    );
    let claims = verifier.verify(&token).await.expect("must verify");
    assert_eq!(claims.sub, "auth0|abc");
    assert_eq!(claims.verified_email.as_deref(), Some("ada@example.com"));
    assert_eq!(claims.connection, "email");
}

fn mock_jwks_from_pem(kid: &str, pem: &[u8]) -> serde_json::Value {
    // Extract n + e from the RSA public PEM via the rsa crate (or shell out
    // — simplest is to commit a precomputed jwks.json next to the keys).
    // For this test we read a precomputed fixture.
    let _ = (kid, pem);
    serde_json::from_str(include_str!("../src/auth/dev_issuer/jwks.json")).unwrap()
}
```

- [ ] **Step 2: Generate the JWKS fixture**

```bash
# Use a small Python one-liner via uv to convert PEM → JWK n+e.
uv run --with cryptography python - <<'PY'
import json, base64
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.rsa import RSAPublicNumbers
pub = serialization.load_pem_public_key(open("crates/api/src/auth/dev_issuer/public_key.pem","rb").read())
nums = pub.public_numbers()
def b64u(i): return base64.urlsafe_b64encode(i.to_bytes((i.bit_length()+7)//8,"big")).rstrip(b"=").decode()
jwks = {"keys":[{"kty":"RSA","kid":"test-kid","use":"sig","alg":"RS256","n":b64u(nums.n),"e":b64u(nums.e)}]}
open("crates/api/src/auth/dev_issuer/jwks.json","w").write(json.dumps(jwks,indent=2))
PY
```

- [ ] **Step 3: Implement the verifier**

Replace `crates/api/src/auth/auth0_jwks.rs`:

```rust
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
```

- [ ] **Step 4: Run the test**

```bash
cargo test -p api --test auth0_jwks
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/auth/auth0_jwks.rs crates/api/src/auth/dev_issuer/jwks.json crates/api/tests/auth0_jwks.rs
git commit -m "feat(api): Auth0 JWKS verifier with 1h cache"
```

---

### Task 7: Replace `resolve_current_player` with the JWT seam middleware

**Files:**
- Create: `crates/api/src/auth/seam.rs`
- Modify: `crates/api/src/router.rs`
- Modify: `crates/api/src/lib.rs` (drop the `X-Dev-Player` reference in the doc-comment)

- [ ] **Step 1: Write the failing test**

Create `crates/api/tests/seam.rs`:

```rust
//! End-to-end: a request with a valid Bearer JWT for a seeded player
//! resolves to `CurrentPlayer::Player`. No header → `Visitor`. Unknown
//! sub → `AuthenticatedUnclaimed`. (Identity→Person→Player resolution
//! itself is covered in Task 11; this test bypasses it for now by
//! plugging `demo-ada` directly into the seam's "sub == player_id"
//! shortcut, which Task 11 replaces with the real lookup.)

mod common;

use api::auth::local_issuer::mint_for_test;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn bearer_token_resolves_to_player() {
    std::env::set_var("LOCAL_AUTH_ISSUER", "1");
    let (app, _repo) = common::test_app().await;

    let token = mint_for_test(common::ALICE, "alice@example.com");
    let req = Request::post("/api/graphql")
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(r#"{"query":"{ me { id } }"}"#))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["data"]["me"]["id"].as_str(), Some(common::ALICE));
}

#[tokio::test]
async fn no_bearer_is_visitor() {
    std::env::set_var("LOCAL_AUTH_ISSUER", "1");
    let (app, _repo) = common::test_app().await;
    let req = Request::post("/api/graphql")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"query":"{ me { id } }"}"#))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v["errors"][0]["message"].as_str().unwrap().contains("authentication required"));
}
```

`common::test_app()` already exists in `tests/common/mod.rs`; if it currently sets up the `X-Dev-Player` flow, update it in Step 3.

- [ ] **Step 2: Run, see it fail**

```bash
cargo test -p api --test seam 2>&1 | head -20
```

Expected: missing exports / no header logic in router.

- [ ] **Step 3: Implement `seam.rs`**

```rust
//! The auth seam axum middleware. Runs after `cloudfront_auth`. Extracts a
//! Bearer token; verifies against the trust-list; resolves to a
//! `CurrentPlayer`; places it (and the `RequestNow`) into request
//! extensions for the GraphQL handler to read.
//!
//! A request with no `Authorization` header is a `Visitor` — no token, no
//! error. An invalid token IS an error (`401`).

use crate::auth::jwt::{verify_token, TrustList, VerifiedClaims};
use crate::auth::resolution::resolve_player;
use crate::auth::{CurrentPlayer, VerifiedIdentity};
use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use storage::Repository;

#[derive(Clone)]
pub struct SeamState {
    pub trust: TrustList,
    pub repo: Arc<dyn Repository>,
}

pub async fn auth_seam(
    State(state): State<SeamState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let current = match bearer {
        None => CurrentPlayer::Visitor,
        Some(token) => match verify_token(&state.trust, token).await {
            Err(_) => return (StatusCode::UNAUTHORIZED, "invalid token").into_response(),
            Ok(claims) => to_current_player(claims, state.repo.as_ref()).await,
        },
    };

    request.extensions_mut().insert(current);
    next.run(request).await
}

async fn to_current_player(claims: VerifiedClaims, repo: &dyn Repository) -> CurrentPlayer {
    // Task 11 replaces this with the full §3 algorithm. Placeholder: if a
    // Player with id == sub exists, that's the player; otherwise unclaimed.
    if let Ok(Some(player)) = repo.get_player(&claims.sub).await {
        CurrentPlayer::Player(Box::new(player))
    } else {
        CurrentPlayer::AuthenticatedUnclaimed(VerifiedIdentity {
            connection: claims.connection,
            sub: claims.sub,
            verified_email: claims.verified_email,
            verified_phone: claims.verified_phone,
        })
    }
}
```

Add `pub mod seam;` to `crates/api/src/auth/mod.rs`. Stub `resolution.rs` for now:

```rust
//! `verified_claims → Identity → Person → Player` resolution. Filled in
//! by Task 11.

use crate::auth::CurrentPlayer;
use crate::auth::jwt::VerifiedClaims;
use storage::Repository;

#[allow(dead_code)]
pub async fn resolve_player(_repo: &dyn Repository, _claims: VerifiedClaims) -> CurrentPlayer {
    unimplemented!("Task 11")
}
```

- [ ] **Step 4: Wire it into the router**

Replace `crates/api/src/router.rs` body (keep the `AppState` struct and the playground / health routes):

```rust
//! The axum router: GraphQL endpoint, playground, health, and the auth
//! seam (`docs/superpowers/specs/2026-05-30-auth-design.md` §2).

use crate::auth::seam::{auth_seam, SeamState};
use crate::auth::CurrentPlayer;
use crate::auth::jwt::TrustList;
use crate::gql::XpoolSchema;
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::{Extension, State},
    http::HeaderMap,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::sync::Arc;
use storage::Repository;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub schema: XpoolSchema,
    pub repo: Arc<dyn Repository>,
}

async fn graphql_handler(
    State(state): State<AppState>,
    Extension(current): Extension<CurrentPlayer>,
    headers: HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let now = crate::clock::RequestNow(crate::clock::resolve_now(&headers));
    let req = req.into_inner().data(current).data(now);
    state.schema.execute(req).await.into()
}

async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new(
        "/api/graphql",
    )))
}

async fn health() -> impl IntoResponse {
    "ok"
}

pub fn build_router(
    schema: XpoolSchema,
    repo: Arc<dyn Repository>,
    cors: bool,
    cloudfront_secret: Option<String>,
) -> Router {
    let state = AppState { schema, repo: repo.clone() };
    let trust = TrustList::from_env();
    let seam_state = SeamState { trust, repo };

    let mut router = Router::new()
        .route(
            "/api/graphql",
            get(graphql_playground).post(graphql_handler),
        )
        .route("/api/health", get(health))
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(
            seam_state,
            auth_seam,
        ));

    if cors {
        router = router.layer(CorsLayer::permissive());
    }
    if let Some(expected) = cloudfront_secret {
        router = router.layer(axum::middleware::from_fn_with_state(
            expected,
            crate::cloudfront_auth::require_cloudfront_secret,
        ));
    }
    router
}
```

- [ ] **Step 5: Run the tests**

```bash
cargo test -p api --test seam
```

Expected: both pass. If `common::test_app()` needs adjustment (e.g. it sets `cloudfront_secret`), update it. Other API tests may now fail because they were sending `X-Dev-Player` — Task 9 fixes them.

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/auth/seam.rs crates/api/src/auth/resolution.rs \
        crates/api/src/auth/mod.rs crates/api/src/router.rs crates/api/src/lib.rs \
        crates/api/tests/seam.rs
git commit -m "feat(api): Bearer-JWT auth seam middleware (drops X-Dev-Player)"
```

---

### Task 8: Dev-login endpoint — `POST /api/dev/login`

**Files:**
- Create: `crates/api/src/auth/dev_login.rs`
- Modify: `crates/api/src/router.rs`
- Modify: `crates/api/src/auth/mod.rs`

- [ ] **Step 1: Write the failing test**

Create `crates/api/tests/dev_login.rs`:

```rust
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn dev_login_returns_a_local_issuer_jwt() {
    std::env::set_var("LOCAL_AUTH_ISSUER", "1");
    let (app, _repo) = common::test_app().await;

    let req = Request::post("/api/dev/login")
        .header("content-type", "application/json")
        .body(Body::from(format!(r#"{{"player":"{}"}}"#, common::ALICE)))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let token = v["token"].as_str().unwrap();
    assert!(!token.is_empty());
}

#[tokio::test]
async fn dev_login_rejects_unknown_player() {
    std::env::set_var("LOCAL_AUTH_ISSUER", "1");
    let (app, _repo) = common::test_app().await;
    let req = Request::post("/api/dev/login")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"player":"nobody"}"#))
        .unwrap();
    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 2: Implement `dev_login.rs`**

```rust
//! `POST /api/dev/login` — mints a local-issuer JWT for a seeded player.
//! Gated behind the `dev_auth` Cargo feature (off in `--features lambda`
//! production builds) AND the `LOCAL_AUTH_ISSUER` env var (off in prod
//! config). Belt-and-suspenders.

use crate::auth::local_issuer::mint_for_test;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use storage::Repository;

#[derive(Clone)]
pub struct DevLoginState {
    pub repo: Arc<dyn Repository>,
}

#[derive(Deserialize)]
pub struct DevLoginRequest {
    pub player: String,
}

#[derive(Serialize)]
pub struct DevLoginResponse {
    pub token: String,
}

pub async fn dev_login(
    State(state): State<DevLoginState>,
    Json(req): Json<DevLoginRequest>,
) -> Response {
    // Verify the player exists. The dev-login endpoint is the only path
    // that pre-existing seeded players is OK — Auth0 doesn't get this
    // shortcut.
    let player = match state.repo.get_player(&req.player).await {
        Ok(Some(p)) => p,
        _ => return (StatusCode::NOT_FOUND, "unknown player").into_response(),
    };
    // Synthesize a verified email from the player id so the resolver
    // (Task 11) can find the corresponding Identity row.
    let email = format!("{}@dev.invalid", player.id);
    let token = mint_for_test(&player.id, &email);
    (StatusCode::OK, Json(DevLoginResponse { token })).into_response()
}
```

- [ ] **Step 3: Mount the route — gated**

In `crates/api/src/router.rs`, add:

```rust
// at the top of build_router, after the other routes:
#[cfg(feature = "dev_auth")]
{
    use crate::auth::dev_login::{dev_login, DevLoginState};
    use axum::routing::post;
    if std::env::var("LOCAL_AUTH_ISSUER").ok().filter(|v| !v.is_empty()).is_some() {
        router = router.route(
            "/api/dev/login",
            post(dev_login).with_state(DevLoginState { repo: repo.clone() }),
        );
    }
}
```

(Adjust the `router` variable scope: move the `let mut router = ...` above this block.) Add `pub mod dev_login;` to `auth/mod.rs` under `#[cfg(feature = "dev_auth")]`.

- [ ] **Step 4: Run the tests**

```bash
cargo test -p api --test dev_login
```

Expected: PASS. Also confirm the prod build excludes the endpoint:

```bash
cargo check -p api --no-default-features --features lambda
```

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/auth/dev_login.rs crates/api/src/auth/mod.rs \
        crates/api/src/router.rs crates/api/tests/dev_login.rs
git commit -m "feat(api): POST /api/dev/login dev-only JWT mint endpoint"
```

---

### Task 9: Migrate `tests/common/mod.rs` and existing API tests to Bearer JWT

**Files:**
- Modify: `crates/api/tests/common/mod.rs`
- Modify: any test in `crates/api/tests/` that sends `X-Dev-Player`

- [ ] **Step 1: Inventory the callers**

```bash
rg -n "x-dev-player|X-Dev-Player" crates/api/tests/
```

- [ ] **Step 2: Add a `with_player` helper to common**

In `crates/api/tests/common/mod.rs`, ensure `LOCAL_AUTH_ISSUER` is set for tests (use a once-init), and add:

```rust
use api::auth::local_issuer::mint_for_test;

pub fn auth_header(player_id: &str) -> (axum::http::HeaderName, axum::http::HeaderValue) {
    std::env::set_var("LOCAL_AUTH_ISSUER", "1");
    let email = format!("{player_id}@dev.invalid");
    let token = mint_for_test(player_id, &email);
    (
        axum::http::header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    )
}

/// Drop-in replacement for the old `x-dev-player` header.
pub fn with_player(builder: axum::http::request::Builder, player_id: &str) -> axum::http::request::Builder {
    let (name, value) = auth_header(player_id);
    builder.header(name, value)
}

/// POST a GraphQL body authenticated as `player_id`, return the parsed
/// response JSON. Used by mutation/query tests.
pub async fn query_as(
    app: &axum::Router,
    player_id: &str,
    body: &str,
) -> serde_json::Value {
    let (name, value) = auth_header(player_id);
    let req = axum::http::Request::post("/api/graphql")
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .header(name, value)
        .body(axum::body::Body::from(body.to_owned()))
        .unwrap();
    let res = tower::ServiceExt::oneshot(app.clone(), req).await.unwrap();
    let bytes = http_body_util::BodyExt::collect(res.into_body())
        .await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Same as `query_as`, but with a pre-minted Bearer token (used when
/// testing the unclaimed / claim flows for arbitrary subs).
pub async fn query_with_bearer(
    app: &axum::Router,
    bearer: &str,
    body: &str,
) -> serde_json::Value {
    let req = axum::http::Request::post("/api/graphql")
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .header(axum::http::header::AUTHORIZATION, format!("Bearer {bearer}"))
        .body(axum::body::Body::from(body.to_owned()))
        .unwrap();
    let res = tower::ServiceExt::oneshot(app.clone(), req).await.unwrap();
    let bytes = http_body_util::BodyExt::collect(res.into_body())
        .await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}
```

- [ ] **Step 3: Replace every `X-Dev-Player` use across `crates/api/tests/`**

For each match from Step 1, replace `.header("X-Dev-Player", "...")` with `.header(common::auth_header(...).0, common::auth_header(...).1)` (or use `with_player`).

- [ ] **Step 4: Run the full API test suite**

```bash
cargo test -p api 2>&1 | tail -30
```

Expected: all tests pass. If any fail, fix the test (not the production code) — they were exercising the X-Dev-Player branch that no longer exists.

- [ ] **Step 5: Commit**

```bash
git add crates/api/tests/
git commit -m "test(api): migrate tests off X-Dev-Player to Bearer JWT"
```

---

### Task 10: Clean up the X-Dev-Player references in doc comments

**Files:**
- Modify: `crates/api/src/clock.rs:9` (doc comment)
- Modify: `crates/api/src/cloudfront_auth.rs:11` (doc comment)
- Modify: `crates/api/src/lib.rs` (doc comment)

- [ ] **Step 1: Replace each reference**

In `clock.rs:9`, replace `\`X-Dev-Player\`` with `\`LOCAL_AUTH_ISSUER\``.
In `cloudfront_auth.rs:11`, replace the parenthetical `X-Dev-Player` mention with `LOCAL_AUTH_ISSUER` and reference the new spec doc.
In `lib.rs`, rewrite the auth-stub paragraph:

```rust
//! xpool API crate — an axum + async-graphql server (`API.md`).
//!
//! The GraphQL layer is a thin adapter: coarse load → expose graph → glue to
//! the pure `domain`/`fwc26` functions. The auth seam (`auth/`) verifies a
//! Bearer JWT (multi-issuer: Auth0 + a local RS256 issuer for dev/tests),
//! resolves `Identity → Person → Player`, and places `CurrentPlayer` in the
//! GraphQL context. See
//! `docs/superpowers/specs/2026-05-30-auth-design.md`.
```

- [ ] **Step 2: Commit**

```bash
git add crates/api/src/
git commit -m "docs(api): drop X-Dev-Player references in module doc-comments"
```

---

## Phase 2 — Identity model & resolution

### Task 11: Add `verified_email` to `domain::Identity`

**Files:**
- Modify: `crates/domain/src/model.rs:141-146`

- [ ] **Step 1: Update the type**

```rust
/// A login credential. Global entity. `verified_email` is the
/// cross-provider match key — when a login arrives via a new provider and
/// its verified email matches an existing `Person` via this field, AUTH-13
/// linking is triggered (spec §3, §6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub id: String,
    pub provider: String,
    pub provider_id: String,
    pub person_id: String,
    pub verified_email: Option<String>,
}
```

- [ ] **Step 2: Fix every construction site**

```bash
rg -n "Identity \{" crates/
```

Add `verified_email: None` (or the appropriate value) to each literal.

- [ ] **Step 3: Build**

```bash
cargo build --workspace
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add crates/
git commit -m "feat(domain): Identity.verified_email (cross-provider match key)"
```

---

### Task 12: `Repository::find_identities_by_verified_email`

**Files:**
- Modify: `crates/storage/src/lib.rs`
- Modify: `crates/storage/src/memory.rs`
- Modify: `crates/storage/src/dynamo.rs`
- Modify: `crates/storage/tests/...` (add a round-trip test)

- [ ] **Step 1: Write the failing test**

In `crates/storage/tests/identity.rs` (create if missing):

```rust
use domain::Identity;
use storage::{InMemoryRepository, Repository};

#[tokio::test]
async fn find_by_verified_email_returns_all_matches() {
    let repo = InMemoryRepository::default();
    repo.put_identity(&Identity {
        id: "i1".into(),
        provider: "email".into(),
        provider_id: "ada@example.com".into(),
        person_id: "p1".into(),
        verified_email: Some("ada@example.com".into()),
    }).await.unwrap();
    repo.put_identity(&Identity {
        id: "i2".into(),
        provider: "google".into(),
        provider_id: "g-123".into(),
        person_id: "p1".into(),
        verified_email: Some("ada@example.com".into()),
    }).await.unwrap();
    repo.put_identity(&Identity {
        id: "i3".into(),
        provider: "email".into(),
        provider_id: "other@example.com".into(),
        person_id: "p2".into(),
        verified_email: Some("other@example.com".into()),
    }).await.unwrap();

    let hits = repo.find_identities_by_verified_email("ada@example.com").await.unwrap();
    assert_eq!(hits.len(), 2);
    let person_ids: std::collections::HashSet<_> =
        hits.iter().map(|i| i.person_id.clone()).collect();
    assert_eq!(person_ids, ["p1".to_string()].into_iter().collect());
}
```

- [ ] **Step 2: Add to the trait**

```rust
async fn find_identities_by_verified_email(
    &self,
    email: &str,
) -> anyhow::Result<Vec<Identity>>;
```

- [ ] **Step 3: Implement for `InMemoryRepository`**

In `memory.rs`, scan the identity store:

```rust
async fn find_identities_by_verified_email(
    &self,
    email: &str,
) -> anyhow::Result<Vec<Identity>> {
    let guard = self.identities.read().await;
    Ok(guard
        .values()
        .filter(|i| i.verified_email.as_deref() == Some(email))
        .cloned()
        .collect())
}
```

- [ ] **Step 4: Implement for `DynamoRepository`**

In `dynamo.rs`, scan the global zone (acceptable for hobby scale). Filter on `verified_email`. Annotate:

```rust
// Linear scan of the identity partition. With ~hundreds of identities at
// hobby scale this is cheap; if scale grows materially, add a GSI on
// `verified_email` and switch to `Query`.
```

- [ ] **Step 5: Run the test (and the DynamoDB variant if `DYNAMO_TEST=1`)**

```bash
cargo test -p storage
DYNAMO_TEST=1 cargo test -p storage
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/storage/
git commit -m "feat(storage): Repository::find_identities_by_verified_email"
```

---

### Task 13: Identity-key helpers + the real resolution algorithm

**Files:**
- Modify: `crates/api/src/auth/resolution.rs`
- Create: `crates/api/tests/resolution.rs`

- [ ] **Step 1: Write the failing tests**

```rust
//! The §3 login-resolution algorithm.

mod common;

use api::auth::jwt::VerifiedClaims;
use api::auth::resolution::{identity_key_for, resolve_player};
use api::auth::{CurrentPlayer, VerifiedIdentity};
use domain::{Identity, Person};

#[test]
fn identity_key_for_email_connection() {
    let claims = VerifiedClaims {
        sub: "auth0|abc".into(),
        verified_email: Some("ada@example.com".into()),
        verified_phone: None,
        connection: "email".into(),
    };
    let (provider, provider_id) = identity_key_for(&claims).unwrap();
    assert_eq!(provider, "email");
    assert_eq!(provider_id, "ada@example.com");
}

#[test]
fn identity_key_for_google_connection() {
    let claims = VerifiedClaims {
        sub: "google-oauth2|123".into(),
        verified_email: Some("ada@example.com".into()),
        verified_phone: None,
        connection: "google".into(),
    };
    let (provider, provider_id) = identity_key_for(&claims).unwrap();
    assert_eq!(provider, "google");
    assert_eq!(provider_id, "google-oauth2|123");
}

#[tokio::test]
async fn resolve_finds_player_via_identity() {
    let (_, repo) = common::test_app().await;
    repo.put_identity(&Identity {
        id: "i1".into(),
        provider: "email".into(),
        provider_id: "alice@example.com".into(),
        person_id: "p-alice".into(),
        verified_email: Some("alice@example.com".into()),
    }).await.unwrap();
    repo.put_person(&Person {
        id: "p-alice".into(),
        identity_ids: vec!["i1".into()],
    }).await.unwrap();
    // The `alice` Player already exists in the common fixture.

    // The Person points to a `Player` whose id equals the Person id —
    // wire that explicitly via the storage's person→player linkage.
    // (Implementation detail of common::test_app; adjust to your fixture.)

    let claims = VerifiedClaims {
        sub: "anything".into(),
        verified_email: Some("alice@example.com".into()),
        verified_phone: None,
        connection: "email".into(),
    };
    let current = resolve_player(repo.as_ref(), claims).await;
    match current {
        CurrentPlayer::Player(p) => assert_eq!(p.id, common::ALICE),
        other => panic!("expected Player, got {other:?}"),
    }
}

#[tokio::test]
async fn resolve_unknown_email_is_unclaimed() {
    let (_, repo) = common::test_app().await;
    let claims = VerifiedClaims {
        sub: "auth0|xyz".into(),
        verified_email: Some("stranger@example.com".into()),
        verified_phone: None,
        connection: "email".into(),
    };
    let current = resolve_player(repo.as_ref(), claims).await;
    assert!(matches!(current, CurrentPlayer::AuthenticatedUnclaimed(_)));
}
```

- [ ] **Step 2: Implement `resolution.rs`**

```rust
//! The §3 login-resolution algorithm.

use crate::auth::jwt::VerifiedClaims;
use crate::auth::{CurrentPlayer, VerifiedIdentity};
use storage::Repository;

/// Returns the `(provider, provider_id)` an Identity row should be keyed
/// at for a given verified claims-set. Returns None when the connection
/// has no usable contact (shouldn't happen with verified claims, but the
/// caller should treat None as "Visitor").
pub fn identity_key_for(claims: &VerifiedClaims) -> Option<(String, String)> {
    match claims.connection.as_str() {
        "email" | "dev" => {
            claims.verified_email.as_ref().map(|e| ("email".to_owned(), e.clone()))
        }
        "sms" => {
            claims.verified_phone.as_ref().map(|p| ("phone".to_owned(), p.clone()))
        }
        "google" => Some(("google".to_owned(), claims.sub.clone())),
        _ => None,
    }
}

/// The full algorithm:
///
/// 1. Look up Identity by (provider, provider_id).
/// 2. Found → Person → Player → return Player.
/// 3. Not found, verified email exists, find_identities_by_verified_email
///    returns hits → AuthenticatedUnclaimed (link path — UI prompts in
///    Task 21).
/// 4. Not found, no email match → AuthenticatedUnclaimed (claim/join
///    path).
/// 5. No verified contact at all → Visitor.
pub async fn resolve_player(
    repo: &dyn Repository,
    claims: VerifiedClaims,
) -> CurrentPlayer {
    let Some((provider, provider_id)) = identity_key_for(&claims) else {
        return CurrentPlayer::Visitor;
    };

    if let Ok(Some(identity)) = repo.get_identity(&provider, &provider_id).await {
        if let Ok(Some(_person)) = repo.get_person(&identity.person_id).await {
            // Person → Player. By the data model's convention, `Player.id`
            // equals `Person.id` for the current tournament.
            if let Ok(Some(player)) = repo.get_player(&identity.person_id).await {
                return CurrentPlayer::Player(Box::new(player));
            }
        }
    }
    // Fall through to unclaimed — the resolver caller (or claim mutation)
    // handles the link-vs-claim disambiguation against `find_identities_by_verified_email`.
    CurrentPlayer::AuthenticatedUnclaimed(VerifiedIdentity {
        connection: claims.connection,
        sub: claims.sub,
        verified_email: claims.verified_email,
        verified_phone: claims.verified_phone,
    })
}
```

- [ ] **Step 3: Swap the seam's placeholder for the real resolver**

In `crates/api/src/auth/seam.rs::to_current_player`, replace the body with:

```rust
crate::auth::resolution::resolve_player(repo, claims).await
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p api --test resolution
cargo test -p api --test seam
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/auth/resolution.rs crates/api/src/auth/seam.rs crates/api/tests/resolution.rs
git commit -m "feat(api): Identity→Person→Player resolution (spec §3)"
```

---

## Phase 3 — Invite codes & claim mutations

### Task 14: Signed invite-code payload

**Files:**
- Create: `crates/api/src/auth/invite_code.rs`
- Create: `crates/api/tests/invite_code.rs`

- [ ] **Step 1: Write the failing test**

```rust
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
```

- [ ] **Step 2: Implement `invite_code.rs`**

```rust
//! Signed invite codes (spec §5).
//!
//! Encoding: `urlsafe_b64(serde_json(payload)).` + `urlsafe_b64(hmac_sha256)`.
//! HS256 with the `INVITE_CODE_SECRET` env var (a 32-byte secret tofu
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
    let expected = mac.finalize().into_bytes();
    let actual = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(sig)?;
    if expected.as_slice() != actual.as_slice() {
        anyhow::bail!("signature mismatch");
    }
    let json = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(body)?;
    let payload: InvitePayload = serde_json::from_slice(&json)?;
    if payload.expires_at < Utc::now() {
        anyhow::bail!("expired code");
    }
    Ok(payload)
}
```

Add to `crates/api/Cargo.toml`:

```toml
hmac = "0.12"
sha2 = "0.10"
```

Register the module in `auth/mod.rs`: `pub mod invite_code;`.

- [ ] **Step 3: Run the tests**

```bash
cargo test -p api --test invite_code
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/api/Cargo.toml crates/api/src/auth/invite_code.rs crates/api/tests/invite_code.rs
git commit -m "feat(api): signed invite-code payload (HS256)"
```

---

### Task 15: `createInvite` GraphQL mutation

**Files:**
- Modify: `crates/api/src/gql/` (locate the existing mutation root)
- Modify: `crates/api/src/gql/mod.rs`
- Create: `crates/api/src/gql/invite.rs`

- [ ] **Step 1: Inventory the existing mutation root**

```bash
rg -n "EmptyMutation|MutationRoot|mutation_root|impl Mutation" crates/api/src/gql/
```

Identify the mutation root struct (e.g. `Mutation`) and where its `submit_group` etc. are declared.

- [ ] **Step 2: Write the failing test**

In `crates/api/tests/invite_mutations.rs`:

```rust
mod common;

use api::auth::invite_code::decode_invite;

#[tokio::test]
async fn create_invite_returns_a_signed_link() {
    std::env::set_var("INVITE_CODE_SECRET", "test-secret-must-be-32-bytes-long");
    let (app, _repo) = common::test_app().await;

    let body = r#"{"query":"mutation { createInvite(pool: null) { code link } }"}"#;
    let res = common::query_as(&app, common::ALICE, body).await;
    let code = res["data"]["createInvite"]["code"].as_str().unwrap().to_string();
    let link = res["data"]["createInvite"]["link"].as_str().unwrap();
    assert!(link.ends_with(&code));
    let payload = decode_invite(b"test-secret-must-be-32-bytes-long", &code).unwrap();
    assert_eq!(payload.referrer, common::ALICE);
    assert!(payload.pool.is_none());
}
```

(Helper `common::query_as` should send a Bearer JWT for the given player id and return the parsed response JSON.)

- [ ] **Step 3: Implement the mutation**

Create `crates/api/src/gql/invite.rs`:

```rust
//! Invite-related GraphQL types and resolvers (spec §5).

use crate::auth::invite_code::{encode_invite, InvitePayload, UsePolicy};
use crate::auth::CurrentPlayer;
use async_graphql::{Context, Object, SimpleObject};
use chrono::{Duration, Utc};

#[derive(SimpleObject)]
pub struct InviteLink {
    /// Just the opaque code, for testing / programmatic use.
    pub code: String,
    /// The full `https://<origin>/invite/<code>` link the inviter shares.
    pub link: String,
}

pub struct InviteMutation;

#[Object]
impl InviteMutation {
    /// Generate a referral invite (or a pool-join invite if `pool` is set).
    async fn create_invite(
        &self,
        ctx: &Context<'_>,
        pool: Option<String>,
    ) -> async_graphql::Result<InviteLink> {
        let me = CurrentPlayer::require(ctx)?;
        let secret = std::env::var("INVITE_CODE_SECRET")
            .map_err(|_| async_graphql::Error::new("INVITE_CODE_SECRET not configured"))?;
        let use_policy = match &pool {
            Some(_) => UsePolicy::MultiUseUntilRotated,
            None => UsePolicy::SingleUse,
        };
        let payload = InvitePayload {
            referrer: me.id.clone(),
            pool,
            expires_at: Utc::now() + Duration::days(30),
            use_policy,
        };
        let code = encode_invite(secret.as_bytes(), &payload)
            .map_err(|e| async_graphql::Error::new(format!("encode failed: {e}")))?;
        let origin = std::env::var("XPOOL_PUBLIC_ORIGIN")
            .unwrap_or_else(|_| "http://localhost:5173".to_owned());
        let link = format!("{origin}/invite/{code}");
        Ok(InviteLink { code, link })
    }
}
```

Wire `InviteMutation` into the GraphQL mutation root using async-graphql's `MergedObject` pattern, or extend the existing `Mutation` root with an `async fn create_invite` that delegates. Match the existing pattern in `crates/api/src/gql/mod.rs`.

- [ ] **Step 4: Run**

```bash
cargo test -p api --test invite_mutations create_invite_returns_a_signed_link
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/gql/invite.rs crates/api/src/gql/ crates/api/tests/invite_mutations.rs
git commit -m "feat(api): createInvite mutation"
```

---

### Task 16: `claimInvite` mutation — lazy Person/Player creation

**Files:**
- Modify: `crates/api/src/gql/invite.rs`
- Modify: `crates/api/tests/invite_mutations.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/invite_mutations.rs`:

```rust
#[tokio::test]
async fn claim_invite_creates_player_with_referrer() {
    std::env::set_var("INVITE_CODE_SECRET", "test-secret-must-be-32-bytes-long");
    std::env::set_var("LOCAL_AUTH_ISSUER", "1");
    let (app, repo) = common::test_app().await;

    // Alice creates an invite.
    let create_body = r#"{"query":"mutation { createInvite(pool: null) { code } }"}"#;
    let res = common::query_as(&app, common::ALICE, create_body).await;
    let code = res["data"]["createInvite"]["code"].as_str().unwrap();

    // A fresh visitor (no matching Identity) authenticates with verified email
    // "newbie@example.com" and claims with that code, supplying nick/full name.
    let token = api::auth::local_issuer::mint_for_test("auth0|newbie", "newbie@example.com");
    let claim_body = format!(
        r#"{{"query":"mutation {{ claimInvite(code: \"{code}\", nick: \"Newbie\", fullName: \"New B.\") {{ player {{ id nick }} }} }}"}}"#,
    );
    let res = common::query_with_bearer(&app, &token, &claim_body).await;
    let player_id = res["data"]["claimInvite"]["player"]["id"].as_str().unwrap();

    let player = repo.get_player(player_id).await.unwrap().unwrap();
    assert_eq!(player.nick, "Newbie");
    assert_eq!(player.referrer.as_deref(), Some(common::ALICE));
}
```

- [ ] **Step 2: Implement the mutation**

Append to `crates/api/src/gql/invite.rs`:

```rust
use crate::auth::invite_code::decode_invite;
use crate::auth::resolution::identity_key_for;
use crate::auth::jwt::VerifiedClaims;
use async_graphql::SimpleObject;
use domain::{Identity, Person, Player};

#[derive(SimpleObject)]
pub struct ClaimResult {
    pub player: PlayerSummary,
}

#[derive(SimpleObject)]
pub struct PlayerSummary {
    pub id: String,
    pub nick: String,
}

#[Object]
impl InviteMutation {
    /// (Existing create_invite stays.)

    async fn claim_invite(
        &self,
        ctx: &Context<'_>,
        code: String,
        nick: String,
        full_name: String,
    ) -> async_graphql::Result<ClaimResult> {
        let viewer = ctx.data_unchecked::<CurrentPlayer>();
        let unclaimed = match viewer {
            CurrentPlayer::AuthenticatedUnclaimed(u) => u.clone(),
            CurrentPlayer::Player(p) => {
                // Already a Player → AUTH-12: optionally add to the pool,
                // never create a duplicate. Implementation: skip the
                // Person/Player creation; if pool is set, add to pool.
                let payload = decode_payload(&code)?;
                if let Some(pool_id) = payload.pool {
                    add_player_to_pool(ctx, &p.id, &pool_id).await?;
                }
                return Ok(ClaimResult {
                    player: PlayerSummary { id: p.id.clone(), nick: p.nick.clone() },
                });
            }
            CurrentPlayer::Visitor => {
                return Err(async_graphql::Error::new("authentication required"));
            }
        };

        let payload = decode_payload(&code)?;
        let repo = ctx.data_unchecked::<std::sync::Arc<dyn storage::Repository>>();

        // AUTH-12 check by verified email.
        if let Some(email) = &unclaimed.verified_email {
            let hits = repo.find_identities_by_verified_email(email).await?;
            if let Some(identity) = hits.into_iter().next() {
                if let Some(player) = repo.get_player(&identity.person_id).await? {
                    if let Some(pool_id) = payload.pool {
                        add_player_to_pool(ctx, &player.id, &pool_id).await?;
                    }
                    return Ok(ClaimResult {
                        player: PlayerSummary { id: player.id.clone(), nick: player.nick },
                    });
                }
            }
        }

        // Otherwise: lazy create Person + Player + Identity, link referrer.
        let (provider, provider_id) =
            identity_key_for_unclaimed(&unclaimed).ok_or_else(|| {
                async_graphql::Error::new("no verified contact on the auth session")
            })?;
        let person_id = uuid::Uuid::new_v4().to_string();
        let identity = Identity {
            id: uuid::Uuid::new_v4().to_string(),
            provider,
            provider_id,
            person_id: person_id.clone(),
            verified_email: unclaimed.verified_email.clone(),
        };
        let person = Person { id: person_id.clone(), identity_ids: vec![identity.id.clone()] };
        let player = Player {
            id: person_id.clone(),
            nick,
            full_name,
            referrer: Some(payload.referrer.clone()),
            // ... existing Player fields (predictions: empty, is_result_user: false, version: 0, etc.)
            ..Player::default_for_new()
        };
        repo.put_identity(&identity).await?;
        repo.put_person(&person).await?;
        repo.put_player(&player).await?;
        if let Some(pool_id) = payload.pool {
            add_player_to_pool(ctx, &player.id, &pool_id).await?;
        }
        Ok(ClaimResult {
            player: PlayerSummary { id: player.id.clone(), nick: player.nick },
        })
    }
}

fn decode_payload(code: &str) -> async_graphql::Result<crate::auth::invite_code::InvitePayload> {
    let secret = std::env::var("INVITE_CODE_SECRET")
        .map_err(|_| async_graphql::Error::new("INVITE_CODE_SECRET not configured"))?;
    crate::auth::invite_code::decode_invite(secret.as_bytes(), code)
        .map_err(|e| async_graphql::Error::new(format!("invalid invite: {e}")))
}

fn identity_key_for_unclaimed(
    u: &crate::auth::VerifiedIdentity,
) -> Option<(String, String)> {
    let claims = VerifiedClaims {
        sub: u.sub.clone(),
        verified_email: u.verified_email.clone(),
        verified_phone: u.verified_phone.clone(),
        connection: u.connection.clone(),
    };
    identity_key_for(&claims)
}

async fn add_player_to_pool(
    _ctx: &Context<'_>,
    _player_id: &str,
    _pool_id: &str,
) -> async_graphql::Result<()> {
    // Reuse the existing `join_pool` mutation's helper. Locate it via
    // `rg -n "fn join_pool" crates/api/src/gql/` and import here.
    todo!("delegate to the existing pool-join helper")
}
```

Replace `Player::default_for_new()` with the appropriate constructor your `Player` exposes; if none exists, build the struct literally (fields can be inferred from `tests/common/mod.rs::seed_player`).

Replace the `add_player_to_pool` stub with the actual helper from the existing pool-join code.

- [ ] **Step 2.5: Resolve the `add_player_to_pool` integration**

```bash
rg -n "fn join_pool|joinPool|add_member|members\.push" crates/api/src/gql/
```

Replace the `todo!()` body with a call to the located helper.

- [ ] **Step 3: Run**

```bash
cargo test -p api --test invite_mutations claim_invite_creates_player_with_referrer
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/gql/invite.rs crates/api/tests/invite_mutations.rs
git commit -m "feat(api): claimInvite mutation (lazy Player creation, AUTH-12 path)"
```

---

### Task 17: AUTH-13 link-prompt query + `confirmLink` mutation

**Files:**
- Modify: `crates/api/src/gql/invite.rs` (or a new `link.rs`)
- Create: test additions

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn me_for_unclaimed_with_email_match_signals_link_path() {
    std::env::set_var("LOCAL_AUTH_ISSUER", "1");
    let (app, repo) = common::test_app().await;
    // alice@example.com already belongs to player ALICE via an Identity row.
    repo.put_identity(&domain::Identity {
        id: "i-alice-google".into(),
        provider: "google".into(),
        provider_id: "google-oauth2|alice".into(),
        person_id: common::ALICE.into(),
        verified_email: Some("alice@example.com".into()),
    }).await.unwrap();

    // Someone logs in via passwordless email to alice@example.com (sub
    // differs, no Identity row for that exact key).
    let token = api::auth::local_issuer::mint_for_test("auth0|new-sub-for-alice", "alice@example.com");
    let body = r#"{"query":"{ me { ... on UnclaimedViewer { linkCandidate { personId provider } } } }"}"#;
    let res = common::query_with_bearer(&app, &token, body).await;
    let candidate = &res["data"]["me"]["linkCandidate"];
    assert_eq!(candidate["personId"], common::ALICE);
    assert_eq!(candidate["provider"], "google");
}
```

(The `me` resolver needs to be a union type or use `__typename` to distinguish `Player` vs `UnclaimedViewer` — match the existing resolver shape; if `me` currently returns `Player?` only, extend it to a union.)

- [ ] **Step 2: Add the `UnclaimedViewer` GraphQL type and union**

In `crates/api/src/gql/me.rs` (or wherever `me` is defined):

```rust
#[derive(SimpleObject)]
pub struct UnclaimedViewer {
    pub email: Option<String>,
    pub phone: Option<String>,
    /// Set when a Person already exists for the verified email via a
    /// different provider — triggers AUTH-13 link prompt in the UI.
    pub link_candidate: Option<LinkCandidate>,
}

#[derive(SimpleObject)]
pub struct LinkCandidate {
    pub person_id: String,
    pub provider: String,
}

#[derive(async_graphql::Union)]
pub enum Viewer {
    Player(Box<domain::Player>),
    Unclaimed(UnclaimedViewer),
}
```

Update the `me` resolver to return `Option<Viewer>` (None for `Visitor`). When the viewer is `AuthenticatedUnclaimed`, populate `link_candidate` by querying `find_identities_by_verified_email`.

- [ ] **Step 3: Add `confirmLink` mutation**

In `invite.rs`:

```rust
#[Object]
impl InviteMutation {
    /// AUTH-13 confirmation. The caller is currently `AuthenticatedUnclaimed`
    /// with a verified email that matches `person_id`. This attaches the
    /// new Identity to that existing Person.
    async fn confirm_link(
        &self,
        ctx: &Context<'_>,
        person_id: String,
    ) -> async_graphql::Result<ClaimResult> {
        let viewer = ctx.data_unchecked::<CurrentPlayer>();
        let unclaimed = match viewer {
            CurrentPlayer::AuthenticatedUnclaimed(u) => u.clone(),
            _ => return Err(async_graphql::Error::new("not in a link-prompt state")),
        };
        let repo = ctx.data_unchecked::<std::sync::Arc<dyn storage::Repository>>();

        // Verify that the verified email matches a Person via some existing
        // Identity row — defense in depth against a hostile client.
        let email = unclaimed.verified_email.as_deref().ok_or_else(|| {
            async_graphql::Error::new("no verified email on the auth session")
        })?;
        let hits = repo.find_identities_by_verified_email(email).await?;
        if !hits.iter().any(|i| i.person_id == person_id) {
            return Err(async_graphql::Error::new(
                "verified email does not belong to that Person",
            ));
        }

        let (provider, provider_id) = identity_key_for_unclaimed(&unclaimed)
            .ok_or_else(|| async_graphql::Error::new("no usable contact"))?;
        let identity = domain::Identity {
            id: uuid::Uuid::new_v4().to_string(),
            provider,
            provider_id,
            person_id: person_id.clone(),
            verified_email: unclaimed.verified_email.clone(),
        };
        repo.put_identity(&identity).await?;

        let mut person = repo.get_person(&person_id).await?.ok_or_else(|| {
            async_graphql::Error::new("person not found")
        })?;
        person.identity_ids.push(identity.id.clone());
        repo.put_person(&person).await?;

        let player = repo
            .get_player(&person_id)
            .await?
            .ok_or_else(|| async_graphql::Error::new("player not found"))?;
        Ok(ClaimResult {
            player: PlayerSummary { id: player.id.clone(), nick: player.nick },
        })
    }
}
```

- [ ] **Step 4: Run**

```bash
cargo test -p api --test invite_mutations
```

Expected: PASS for all invite_mutations tests.

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/gql/ crates/api/tests/
git commit -m "feat(api): AUTH-13 linkCandidate + confirmLink mutation"
```

---

## Phase 4 — Frontend dev mode

### Task 18: Replace `devAuth.ts` with JWT storage + `devLogin` call

**Files:**
- Modify: `web/src/auth/devAuth.ts`
- Modify: `web/src/auth/AuthContext.tsx`
- Modify: `web/src/auth/authContextValue.ts`

- [ ] **Step 1: Rewrite `devAuth.ts`**

```ts
/**
 * Dev auth. Mints a local-issuer JWT against the API's `/api/dev/login`
 * endpoint and stashes it in localStorage. The urql client reads the JWT
 * on every request and sends `Authorization: Bearer <jwt>`.
 *
 * Production builds use Auth0 SPA SDK instead — see auth0Provider.tsx.
 */

const TOKEN_KEY = 'xpool.jwt'
const PLAYER_KEY = 'xpool.devPlayer'

export function getToken(): string | null {
  try { return localStorage.getItem(TOKEN_KEY) } catch { return null }
}

export function getDevPlayerLabel(): string | null {
  try { return localStorage.getItem(PLAYER_KEY) } catch { return null }
}

export async function devLogin(playerId: string): Promise<string> {
  const res = await fetch('/api/dev/login', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ player: playerId }),
  })
  if (!res.ok) throw new Error(`dev-login failed: ${res.status}`)
  const { token } = (await res.json()) as { token: string }
  try {
    localStorage.setItem(TOKEN_KEY, token)
    localStorage.setItem(PLAYER_KEY, playerId)
  } catch { /* ignore */ }
  return token
}

export function clearToken(): void {
  try {
    localStorage.removeItem(TOKEN_KEY)
    localStorage.removeItem(PLAYER_KEY)
  } catch { /* ignore */ }
}

// The dev-clock storage stays as-is; export the same helpers.
const NOW_KEY = 'xpool.devNow'
export function getDevNow(): string | null {
  try { return localStorage.getItem(NOW_KEY) } catch { return null }
}
export function setDevNow(iso: string): void {
  try { localStorage.setItem(NOW_KEY, iso) } catch { /* ignore */ }
}
export function clearDevNow(): void {
  try { localStorage.removeItem(NOW_KEY) } catch { /* ignore */ }
}
```

- [ ] **Step 2: Update `AuthContext.tsx` to use the new helpers**

```tsx
import { useMemo, useState, type ReactNode } from 'react'
import { clearToken, devLogin as apiDevLogin, getDevPlayerLabel } from './devAuth'
import { AuthContext, type AuthState } from './authContextValue'

export function AuthProvider({ children }: { children: ReactNode }) {
  const [label, setLabel] = useState<string | null>(getDevPlayerLabel())

  const value = useMemo<AuthState>(
    () => ({
      label,
      login: async (id: string) => {
        await apiDevLogin(id)
        setLabel(id)
      },
      logout: () => {
        clearToken()
        setLabel(null)
      },
    }),
    [label],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}
```

- [ ] **Step 3: Update `authContextValue.ts`**

```ts
import { createContext } from 'react'

export type AuthState = {
  /** A display label for the currently-active player (dev mode); null = visitor. */
  label: string | null
  login: (playerId: string) => Promise<void>
  logout: () => void
}

export const AuthContext = createContext<AuthState | null>(null)
```

- [ ] **Step 4: Run the lint + type-check**

```bash
cd web && npm run lint && npx tsc -b
```

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add web/src/auth/
git commit -m "feat(web): dev-login uses JWT + Authorization Bearer"
```

---

### Task 19: urql client sends Bearer JWT

**Files:**
- Modify: `web/src/graphql/client.ts`

- [ ] **Step 1: Replace the header logic**

```ts
import { Client, cacheExchange, fetchExchange } from 'urql'
import { getToken, getDevNow } from '../auth/devAuth'

export function createGraphqlClient(): Client {
  return new Client({
    url: '/api/graphql',
    preferGetMethod: false,
    exchanges: [cacheExchange, fetchExchange],
    fetchOptions: () => {
      const token = getToken()
      const headers: Record<string, string> = { 'content-type': 'application/json' }
      if (token) headers['Authorization'] = `Bearer ${token}`
      const devNow = getDevNow()
      if (devNow) headers['X-Dev-Now'] = devNow
      return { headers }
    },
  })
}
```

- [ ] **Step 2: Run type-check**

```bash
cd web && npx tsc -b
```

Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add web/src/graphql/client.ts
git commit -m "feat(web): send Authorization: Bearer to /api/graphql"
```

---

### Task 20: AuthBar picker uses the new `login`

**Files:**
- Modify: `web/src/components/AuthBar.tsx`

- [ ] **Step 1: Inspect existing AuthBar**

```bash
cat web/src/components/AuthBar.tsx | head -60
```

- [ ] **Step 2: Replace any `setDevPlayerId` / `setPlayerId` usage with `await auth.login(id)`**

The select's `onChange` should call `await login(playerId)`. The "logout" button calls `logout()`. The "Logged in as" text uses `label`.

- [ ] **Step 3: Type-check + run dev server smoke**

```bash
cd web && npx tsc -b
```

Then in another terminal, with the local API already running (per the user's setup, the dev stack is up): visit `http://localhost:5173`, pick a player, confirm the urql client sends `Authorization: Bearer ...` (Network panel).

- [ ] **Step 4: Commit**

```bash
git add web/src/components/AuthBar.tsx
git commit -m "feat(web): AuthBar picker calls the dev-login endpoint"
```

---

### Task 21: e2e — update `devLogin` helper + update auth spec

**Files:**
- Modify: `web/e2e/helpers.ts`
- Modify: `web/e2e/auth.spec.ts`
- Modify: `web/scripts/e2e-stack.sh` (export `LOCAL_AUTH_ISSUER`, `INVITE_CODE_SECRET`)

- [ ] **Step 1: Export the env vars**

In `web/scripts/e2e-stack.sh`, near the existing `export` block:

```bash
export LOCAL_AUTH_ISSUER="${LOCAL_AUTH_ISSUER:-1}"
export INVITE_CODE_SECRET="${INVITE_CODE_SECRET:-test-secret-must-be-32-bytes-long}"
```

- [ ] **Step 2: Update `devLogin` helper**

In `web/e2e/helpers.ts`, the existing `devLogin` picks from a `<select>` — same UX works, but the helper now exercises the JWT path because of the changes in Tasks 18-20. No change to the helper signature; verify the assertion `'.auth-bar' contains 'Logged in as'` still holds.

- [ ] **Step 3: Add a "visitor with invalid token is rejected" test**

In `web/e2e/auth.spec.ts`:

```ts
test('an invalid Bearer token is rejected by the API', async ({ page }) => {
  await page.goto('/')
  await page.evaluate(() => localStorage.setItem('xpool.jwt', 'not.a.jwt'))
  await page.reload()
  // The error view, not a crash. The seam returns 401 → urql surfaces an error.
  const net = watchNetwork(page)
  await page.goto('/profile')
  await expect(page.getByText('Login required')).toBeVisible()
  // No new test added to the schema; we just confirm the integration handles
  // the failure gracefully.
})
```

- [ ] **Step 4: Run the suite**

```bash
cd web && npm run e2e
```

Expected: all tests pass. If any pre-existing test relied on `X-Dev-Player`, update it.

- [ ] **Step 5: Commit**

```bash
git add web/e2e/ web/scripts/e2e-stack.sh
git commit -m "test(web): e2e auth flows via dev-login + Bearer JWT"
```

---

## Phase 5 — Frontend Auth0 integration (production path)

### Task 22: Add `@auth0/auth0-react`

**Files:**
- Modify: `web/package.json`

- [ ] **Step 1: Install**

```bash
cd web && npm install @auth0/auth0-react
```

- [ ] **Step 2: Commit**

```bash
git add web/package.json web/package-lock.json
git commit -m "chore(web): add @auth0/auth0-react"
```

---

### Task 23: `Auth0Provider` wrapper gated by `VITE_AUTH0_DOMAIN`

**Files:**
- Create: `web/src/auth/auth0Provider.tsx`
- Modify: `web/src/main.tsx` (or wherever `<App />` is mounted)
- Modify: `web/src/auth/devAuth.ts` (add `setTokenFromAuth0(token)` helper)

- [ ] **Step 1: Implement the gated provider**

```tsx
import { ReactNode } from 'react'
import { Auth0Provider as SdkProvider, useAuth0 } from '@auth0/auth0-react'
import { useEffect } from 'react'
import { setTokenFromAuth0 } from './devAuth'

const DOMAIN = import.meta.env.VITE_AUTH0_DOMAIN
const CLIENT = import.meta.env.VITE_AUTH0_CLIENT_ID
const AUDIENCE = import.meta.env.VITE_AUTH0_AUDIENCE

export function Auth0Gate({ children }: { children: ReactNode }) {
  if (!DOMAIN || !CLIENT) return <>{children}</>  // dev mode: no Auth0.
  return (
    <SdkProvider
      domain={DOMAIN}
      clientId={CLIENT}
      authorizationParams={{
        redirect_uri: window.location.origin,
        audience: AUDIENCE,
      }}
      cacheLocation="memory"
    >
      <TokenBridge>{children}</TokenBridge>
    </SdkProvider>
  )
}

/** Sync the Auth0 access token into the same storage the urql client reads. */
function TokenBridge({ children }: { children: ReactNode }) {
  const { isAuthenticated, getAccessTokenSilently } = useAuth0()
  useEffect(() => {
    if (!isAuthenticated) return
    let cancelled = false
    void getAccessTokenSilently().then((t) => { if (!cancelled) setTokenFromAuth0(t) })
    return () => { cancelled = true }
  }, [isAuthenticated, getAccessTokenSilently])
  return <>{children}</>
}
```

- [ ] **Step 2: Add `setTokenFromAuth0` to `devAuth.ts`**

```ts
export function setTokenFromAuth0(token: string): void {
  try { localStorage.setItem(TOKEN_KEY, token) } catch { /* ignore */ }
}
```

- [ ] **Step 3: Wrap the app**

In `web/src/main.tsx`, wrap the existing `<App />` (or the route provider) with `<Auth0Gate>`. Order: `<Auth0Gate><AuthProvider>...</AuthProvider></Auth0Gate>`.

- [ ] **Step 4: Type-check**

```bash
cd web && npx tsc -b
```

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add web/src/auth/auth0Provider.tsx web/src/main.tsx web/src/auth/devAuth.ts
git commit -m "feat(web): Auth0 SPA SDK provider gated by VITE_AUTH0_DOMAIN"
```

---

### Task 24: AuthBar — production "Log in" / "Log out" buttons

**Files:**
- Modify: `web/src/components/AuthBar.tsx`

- [ ] **Step 1: Detect mode + render**

```tsx
import { useAuth0 } from '@auth0/auth0-react'
const auth0Enabled = !!import.meta.env.VITE_AUTH0_DOMAIN

export function AuthBar() {
  if (auth0Enabled) return <ProdAuthBar />
  return <DevAuthBar />  // the existing player-picker UI.
}

function ProdAuthBar() {
  const { isAuthenticated, loginWithRedirect, logout, user } = useAuth0()
  if (!isAuthenticated) {
    return (
      <div className="auth-bar">
        You are outside.{' '}
        <button onClick={() => loginWithRedirect()}>Log in</button>
      </div>
    )
  }
  return (
    <div className="auth-bar">
      Logged in as {user?.name ?? user?.email}{' '}
      <button onClick={() => logout({ logoutParams: { returnTo: window.location.origin } })}>
        Log out
      </button>
    </div>
  )
}
```

Move the existing dev picker into `DevAuthBar` (rename the current AuthBar body).

- [ ] **Step 2: Commit**

```bash
git add web/src/components/AuthBar.tsx
git commit -m "feat(web): AuthBar prod mode (Auth0 loginWithRedirect / logout)"
```

---

## Phase 6 — Frontend invite-link UI

### Task 25: Invite page generates a copyable link

**Files:**
- Modify: `web/src/pages/InvitePage.tsx`

- [ ] **Step 1: Wire the `createInvite` mutation**

```tsx
import { useMutation } from 'urql'

const CREATE_INVITE = `mutation { createInvite(pool: null) { code link } }`

export function InvitePage() {
  const [result, run] = useMutation(CREATE_INVITE)
  const link = result.data?.createInvite?.link
  return (
    <main className="content">
      <h2>Invite</h2>
      <button onClick={() => run({})}>Generate link</button>
      {link && (
        <>
          <p>Share this link with your friend:</p>
          <textarea readOnly value={link} onFocus={(e) => e.currentTarget.select()} />
          <button onClick={() => navigator.clipboard.writeText(link)}>Copy</button>
        </>
      )}
    </main>
  )
}
```

- [ ] **Step 2: Commit**

```bash
git add web/src/pages/InvitePage.tsx
git commit -m "feat(web): InvitePage generates and copies the invite link"
```

---

### Task 26: `/invite/:code` claim landing page

**Files:**
- Create: `web/src/pages/InviteClaimPage.tsx`
- Modify: `web/src/App.tsx` (add the route)

- [ ] **Step 1: Implement the claim page**

```tsx
import { useParams } from 'react-router-dom'
import { useMutation, useQuery } from 'urql'
import { useState } from 'react'

const ME = `query { me { __typename ... on Player { id nick } ... on UnclaimedViewer { email linkCandidate { personId provider } } } }`
const CLAIM = `mutation Claim($code: String!, $nick: String!, $fullName: String!) {
  claimInvite(code: $code, nick: $nick, fullName: $fullName) { player { id nick } }
}`

export function InviteClaimPage() {
  const { code } = useParams<{ code: string }>()
  const [me] = useQuery({ query: ME })
  const [, run] = useMutation(CLAIM)
  const [nick, setNick] = useState('')
  const [fullName, setFullName] = useState('')

  if (!code) return <p>missing invite code</p>
  if (me.fetching) return null
  const viewer = me.data?.me
  if (viewer?.__typename === 'Player') {
    // Already a Player; the seam will have handled AUTH-12 if there's a pool.
    return <p>You're already in xPool — welcome back.</p>
  }
  if (!viewer) {
    // Visitor — kick to login (Auth0 redirect, or dev mode picker).
    return <a href="/login">Log in to claim this invite</a>
  }
  // Unclaimed → form.
  return (
    <main className="content">
      <h2>Claim your invite</h2>
      <p>Set your display name.</p>
      <input value={nick} onChange={(e) => setNick(e.target.value)} placeholder="Nick" />
      <input value={fullName} onChange={(e) => setFullName(e.target.value)} placeholder="Full name" />
      <button
        onClick={async () => {
          await run({ code, nick, fullName })
          window.location.href = '/profile'
        }}
      >
        Claim
      </button>
    </main>
  )
}
```

- [ ] **Step 2: Register the route**

In `web/src/App.tsx`, add:

```tsx
<Route path="/invite/:code" element={<InviteClaimPage />} />
```

- [ ] **Step 3: Type-check**

```bash
cd web && npx tsc -b
```

- [ ] **Step 4: Commit**

```bash
git add web/src/pages/InviteClaimPage.tsx web/src/App.tsx
git commit -m "feat(web): /invite/:code claim landing page"
```

---

### Task 27: AUTH-13 link-confirmation prompt UI

**Files:**
- Modify: `web/src/pages/InviteClaimPage.tsx` (handle the `linkCandidate` branch)

- [ ] **Step 1: Branch on `linkCandidate`**

Extend the InviteClaimPage to render a confirmation when `viewer.linkCandidate` is set, before showing the claim form:

```tsx
const LINK = `mutation Link($personId: String!) { confirmLink(personId: $personId) { player { id nick } } }`

// inside the unclaimed branch, before the form:
if (viewer.linkCandidate) {
  return (
    <main className="content">
      <h2>Link this login?</h2>
      <p>
        An account already exists for {viewer.email}, signed in via{' '}
        {viewer.linkCandidate.provider}. Link this login to that account?
      </p>
      <button
        onClick={async () => {
          await run(LINK, { personId: viewer.linkCandidate!.personId })
          window.location.href = '/profile'
        }}
      >
        Yes, link
      </button>
      <button onClick={() => window.location.href = '/'}>
        No, log out
      </button>
    </main>
  )
}
```

- [ ] **Step 2: Run + commit**

```bash
cd web && npx tsc -b
git add web/src/pages/InviteClaimPage.tsx
git commit -m "feat(web): AUTH-13 link-confirmation prompt on claim page"
```

---

### Task 28: AUTH-06 "you need an invitation" state

**Files:**
- Modify: `web/src/pages/HomePage.tsx` (or the auth-bar) — show a banner when viewer is Unclaimed AND no `linkCandidate`

- [ ] **Step 1: Add the banner**

In whatever component subscribes to `me`, when `viewer.__typename === 'UnclaimedViewer'` and no `linkCandidate`, render:

```tsx
<div className="banner">
  You're signed in, but you need an invitation to play.
  Ask a friend who plays for an invite link.
</div>
```

- [ ] **Step 2: Commit**

```bash
git add web/src/
git commit -m "feat(web): AUTH-06 unclaimed banner"
```

---

### Task 29: e2e — invite link end-to-end

**Files:**
- Modify: `web/e2e/auth.spec.ts`

- [ ] **Step 1: Add the test**

```ts
test('a generated invite link claims a new player with referrer set', async ({ page, context }) => {
  // Inviter creates a link.
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.getByRole('link', { name: 'Invite' }).click()
  await page.getByRole('button', { name: 'Generate link' }).click()
  const link = await page.locator('textarea').inputValue()
  expect(link).toMatch(/\/invite\//)

  // A fresh browser session opens the link.
  const fresh = await context.newPage()
  // Override the dev-login: mint a JWT for a new sub via the API directly.
  const res = await fresh.request.post('/api/dev/login', {
    data: { player: 'demo-grace' },  // any seeded player; the resolver will treat the new sub as unclaimed if we drop the linkage. For the test, we mint a custom token via a helper that bypasses player-existence — TODO: extend the helper.
  })
  // (Or: use a test endpoint that mints a token for an arbitrary sub/email.)
  // ...
})
```

The full e2e is invasive — it needs a way to authenticate as a brand-new identity. Two reasonable options:

- Extend the dev-login endpoint to accept `{ sub, email }` directly (test-only).
- Or run the test against the real Auth0 dev tenant (heavier).

For this plan, take the **dev-login extension** path:

- [ ] **Step 2: Extend `/api/dev/login` to accept an explicit sub+email**

In `crates/api/src/auth/dev_login.rs`, replace the request shape:

```rust
#[derive(Deserialize)]
pub struct DevLoginRequest {
    /// Either an existing seeded player id, or — combined with `email` — an
    /// arbitrary unclaimed sub for testing the AUTH-06 / claim flows.
    pub player: Option<String>,
    pub sub: Option<String>,
    pub email: Option<String>,
}
```

Logic: if `player` is set, mint for that player; if `sub` + `email` are set, mint directly. Otherwise 400.

- [ ] **Step 3: Use it from the e2e test** and finish the invite-link claim journey end-to-end. Commit when green.

```bash
cd web && npm run e2e -- auth.spec.ts
git add crates/api/src/auth/dev_login.rs web/e2e/auth.spec.ts
git commit -m "test(web): invite-link end-to-end (dev-login arbitrary-sub mode)"
```

---

## Phase 7 — `.specs/` corrections

### Task 30: `.specs/DATA_MODEL.md`

**Files:**
- Modify: `.specs/DATA_MODEL.md`

- [ ] **Step 1: §12 "Open / deferred"**

Delete the bullet `Auth mechanism — deferred (Auth0 vs app-managed). Phase 1 uses a dev stub behind a single auth seam: ...`. Replace with a one-line cross-reference: `Auth — decided. See \`docs/superpowers/specs/2026-05-30-auth-design.md\`.`

- [ ] **Step 2: §3 entity table — Identity row**

In the row for `Identity`, change `(Google sub, email+password, magic-link)` to `(passwordless email, passwordless phone, Google sub)`.

- [ ] **Step 3: Commit**

```bash
git add .specs/DATA_MODEL.md
git commit -m "docs(specs): DATA_MODEL §12/§3 reflect auth decision"
```

---

### Task 31: `.specs/API.md` §8

**Files:**
- Modify: `.specs/API.md`

- [ ] **Step 1: Rewrite §8 "Auth in the contract"**

Replace the current paragraph with:

```markdown
## 8. Auth in the contract

The edge verifies a Bearer JWT (multi-issuer: Auth0 + a local RS256 issuer for
dev/tests), resolves `Identity → Person → Player`, and places a three-state
`CurrentPlayer` (`Visitor` / `AuthenticatedUnclaimed` / `Player`) in the
GraphQL context. Resolvers read it from context and never re-authenticate.
The dev mechanism is a local-issuer JWT (the `LOCAL_AUTH_ISSUER` env var
toggles trust; a `POST /api/dev/login` endpoint mints tokens). Validation is
in-app, layered behind the existing `cloudfront_auth` middleware. See
`docs/superpowers/specs/2026-05-30-auth-design.md` §§2-3.
```

- [ ] **Step 2: Commit**

```bash
git add .specs/API.md
git commit -m "docs(specs): API.md §8 reflects Bearer-JWT auth seam"
```

---

### Task 32: `.specs/SCENARIOS.md`

**Files:**
- Modify: `.specs/SCENARIOS.md`

- [ ] **Step 1: Update the "Design decisions baked in" Auth bullet**

```markdown
- **Auth**: Auth0 managed IdP, fully passwordless — passwordless email (magic
  link via SES), passwordless SMS (OTP code via Twilio), Google. **No
  email+password.** `Identity → Person → Player`; the **Person layer owns
  identity-linking** (explicit confirmation, never silent). See
  `docs/superpowers/specs/2026-05-30-auth-design.md`.
```

- [ ] **Step 2: AUTH-01 / AUTH-02 — reword the dev mechanism**

In AUTH-01 replace `an \`X-Dev-Player\` header naming a seeded player` with `a Bearer JWT minted by \`POST /api/dev/login\` for a seeded player (the local-issuer path)`. In AUTH-02 replace `a request with no \`X-Dev-Player\` header` with `a request with no Bearer token`.

- [ ] **Step 3: AUTH-05 → `dropped`**

Change `Status: future` to `Status: dropped`. Update the rationale:

```markdown
### AUTH-05 — Login via email + password
Status: dropped · Actor: — · Screen: —
**Dropped** by the 2026-05-30 auth design — passwordless throughout. Removing
passwords removes the only justification for a managed-IdP password store and
reconciles the "don't own password security" and "avoid lock-in" drivers.
```

- [ ] **Step 4: New AUTH-18 — passwordless SMS**

Append to the AUTH section:

```markdown
### AUTH-18 — Login via passwordless SMS
Status: future · Actor: Player · Screen: Login
Given  a `Person` has a linked phone `Identity`.
When   they request a code, receive it via Twilio, and type it.
Then   Auth0 verifies the OTP; the app resolves
       `Identity#phone#<E.164> → Person → Player` and the player is logged in.
Tests: —
Note:  SMS passwordless is a typed code, not a clickable link — a phone can't
       reliably deep-link back into a browser session.
```

- [ ] **Step 5: AUTH-07 / AUTH-08 / AUTH-09 / AUTH-10 / AUTH-11**

Rewrite per the spec's §7 (apply the changes listed there verbatim — eager → lazy, email → shareable signed link, AUTH-08 claim-time, AUTH-10 simplified, AUTH-11 folded). Update the test pointers where the auth flows have working tests now.

- [ ] **Step 6: Commit**

```bash
git add .specs/SCENARIOS.md
git commit -m "docs(specs): SCENARIOS.md auth scenarios reflect 2026-05-30 design"
```

---

## Phase 8 — Auth0 tenant runbook (manual config)

### Task 33: Document Auth0 tenant + application setup

**Files:**
- Create: `docs/runbooks/auth0-setup.md`

- [ ] **Step 1: Write the runbook**

```markdown
# Auth0 setup — xpool

This is a one-time manual checklist. Auth0 configuration is not Terraform-
managed (overkill for one tenant at hobby scale).

## Tenant
- Create one tenant: `xpool`.
- Region: closest to ca-central-1 — `EU` or `US`, your call.

## Application
- Type: **Single-Page Application**.
- Name: `xpool`.
- Allowed Callback URLs:
  - `https://pool.xczimi.com`
  - `https://pool-dev.xczimi.com`
  - `http://localhost:5173`
- Allowed Logout URLs: same three.
- Allowed Web Origins: same three.

## API
- Identifier (audience): `xpool-api`.
- Signing algorithm: RS256.

## Connections
1. **Passwordless email**
   - Custom email provider: SES (`xczimi.com`).
   - Template: link (not code).
2. **Passwordless SMS**
   - Provider: Twilio (your account SID + auth token).
   - Code length: 6.
3. **Google social**
   - Use Auth0's dev keys for first wiring; replace with your own Google
     OAuth credentials before going live.

## Environment variables (per deployment)
- API Lambda:
  - `AUTH0_DOMAIN=<tenant>.auth0.com`
  - `AUTH0_AUDIENCE=xpool-api`
  - `INVITE_CODE_SECRET=<32-byte secret>` (generate per env, store via SSM)
- SPA build:
  - `VITE_AUTH0_DOMAIN`
  - `VITE_AUTH0_CLIENT_ID`
  - `VITE_AUTH0_AUDIENCE=xpool-api`
  - `VITE_XPOOL_PUBLIC_ORIGIN` (used by `createInvite` if you ever generate
    links server-side from the SPA build).

## What's NOT in Auth0
- Sessions: stateless Bearer JWT, validated per request by the API.
- Invite codes: HS256-signed, owned entirely by the app.
- Pending players: lazy — they don't exist until claim.
```

- [ ] **Step 2: Commit**

```bash
git add docs/runbooks/auth0-setup.md
git commit -m "docs: Auth0 tenant + application setup runbook"
```

---

## Final verification

- [ ] Run the full Rust suite: `cargo test --workspace`.
- [ ] Run the full e2e suite: `cd web && npm run e2e`.
- [ ] Confirm prod-style build excludes dev-login: `cargo build -p api --no-default-features --features lambda` succeeds and the resulting binary does not contain the string `dev/login` (`strings target/.../api | rg dev/login` should return nothing).
- [ ] Confirm `LOCAL_AUTH_ISSUER` unset → API rejects all Bearer tokens (`curl -H "Authorization: Bearer ..."` returns 401).

---

## Self-review notes

- Every spec section (§1–§7 + the two unparked addenda) maps to at least one phase: §1/§2 → Phases 1, 5, 7; §3 → Phase 2; §4 → Phases 4, 5; §5 → Phase 3, 6; §6 → Phases 3, 6; §7 → Phase 7; the "Auth0 origins" addendum → Phase 8.
- The plan defers `Player::default_for_new()` to inspection-time — engineers will find the right constructor in the existing seed code (`tests/common/mod.rs`).
- The plan deliberately keeps `cloudfront_auth` untouched — the deployment workstream's middleware is correct and orthogonal.
