# Authentication — design

**Date:** 2026-05-30
**Status:** approved (design); implementation pending
**Workstream:** AUTH (parallel to DEPLOYMENT — both unblocked as of merge of #4)

## Goal

Replace the dev-stub auth (the `X-Dev-Player` header) with real authentication
sufficient for production. Honour the existing auth seam — the edge resolves
`Identity → Person → Player` and places a `CurrentPlayer` in GraphQL context;
resolvers never re-authenticate (see `.specs/API.md` §8, `.specs/DATA_MODEL.md`
§12, `.specs/SCENARIOS.md` AUTH-01..17).

## Decision

**Auth0 as the managed IdP, brokering everything, fully passwordless.**

The choice between Auth0 and app-managed was grilled with three drivers in
mind — don't own password security, avoid vendor lock-in, keep signup/login
simple. They reconcile by **eliminating passwords entirely**: there is no
password-hash database for anyone to own and no password-hash database trapped
inside Auth0, so Auth0 lock-in is shallow by construction. Going passwordless
is what makes "Auth0" and "avoid vendor lock-in" both true at once. The
`Identity → Person → Player` seam remains the swap point.

## §1 — Methods

All three Auth0 connections are passwordless:

- **Passwordless email** — Auth0 sends a magic link, delivered through *our*
  SES (`xczimi.com`) configured as Auth0's custom email provider.
- **Passwordless SMS** — Auth0 sends a 6-digit one-time code, delivered
  through *our* Twilio configured into Auth0's SMS connection. ⚠️ SMS is a
  typed OTP code, not a clickable link; a phone can't reliably deep-link back
  into a browser session. Email stays a true magic link.
- **Google** — Auth0 social connection.

**No email+password, ever.** Verified email/phone is the security boundary
(per AUTH-09). No app-stored credential of any kind; the legacy `authcode`
token stays dropped.

## §2 — The auth seam in the API

The seam does exactly one thing: take a Bearer JWT → verify it → extract
verified `sub` / email / phone → resolve `Identity → Person → Player` → put
`CurrentPlayer` in GraphQL context. **No `X-Dev-Player` header, no dev-only
branch.** One auth code path everywhere; only the signing key varies.

- **Multi-issuer verification.** The seam is configured with a set of trusted
  token issuers, each gated by its own env var:
  - **`AUTH0_DOMAIN`** present → trust Auth0 (verify via its JWKS URL).
    Required in production; unset locally.
  - **`LOCAL_AUTH_ISSUER`** present → trust a local RS256 issuer (verify via
    a static local public key). Required for the dev-login flow and tests;
    unset in production.
  - Both env vars empty → no Bearer token is accepted; everyone is a
    `Visitor`. Mis-configuration fails closed.
  - Both env vars set is supported and benign (e.g. dev exercising the real
    Auth0 flow alongside the dev-login endpoint), but not the default
    anywhere. Same verification routine for both — only the key differs.
- **Minting local tokens:**
  - **Unit / integration tests** mint local-issuer JWTs directly with
    arbitrary claims (claimed player, `AuthenticatedUnclaimed`, result-user,
    expired, wrong audience) and run them through the real production
    verification + resolution code. The seam finally gets genuine test
    coverage.
  - **Local full-stack dev:** a `dev`-gated `POST /api/dev/login { player }`
    endpoint mints a local-issuer JWT for a seeded player; the SPA in dev
    mode calls it instead of the Auth0 redirect. e2e's `global-setup.ts`
    uses the same endpoint.
- **Three-state `CurrentPlayer`** — the current two-state shape can't express
  AUTH-06 (authenticated but no invitation). New shape:
  - `Visitor` — no / invalid token.
  - `AuthenticatedUnclaimed` — valid token, verified email/phone, but no
    `Person`/`Player`. Carries the verified identity so the claim/join flows
    can act on it. Player-only resolvers still return the auth error; this is
    a "logged-in visitor."
  - `Player` — resolved `Player` (incl. the result-user).

### Validation topology (resolved)

Validation is **in-app**, layered behind the existing
`cloudfront_auth::require_cloudfront_secret` middleware
(`crates/api/src/cloudfront_auth.rs`). Two complementary axum middlewares,
separate concerns:

```
request → cloudfront_auth (network gate: came through our CloudFront?)
        → jwt verify + Identity→Person→Player (auth seam: who are you?)
        → graphql handler (reads CurrentPlayer from context)
```

The seam follows the same env-var-presence pattern as `cloudfront_auth` and
`clock.rs` (the trusted-issuer set is whichever of `AUTH0_DOMAIN` /
`LOCAL_AUTH_ISSUER` are populated), so `cargo run -p api` with no env keeps
working identically. No API Gateway, no Lambda authorizer.

## §3 — Identity model & login resolution

`.specs/DATA_MODEL.md` §9 already gives the key shape —
`IDENTITY#<provider>#<providerId>` → `person_id`. This pins down the three
providers and the resolution algorithm.

### Keying scheme

| Connection | `Identity` key | Why |
|---|---|---|
| Passwordless email | `IDENTITY#email#<verified address>` | Known at invite time, before any Auth0 `sub` exists (AUTH-07). |
| Passwordless SMS | `IDENTITY#phone#<E.164>` | Same — keyed on the verified contact. |
| Google | `IDENTITY#google#<google sub>` | The Google `sub` is the stable account id; the verified email is stored as an attribute. |

The Auth0 per-connection `sub` is stored as an `Identity` attribute for audit,
but **resolution keys on the verified contact**, not the `sub`. The verified
email/phone is the security boundary (AUTH-09), it is the only thing known at
invite time, and it is what cross-provider linking matches on (AUTH-13). Every
`Identity` row also records its **verified email** when it has one — that is
the cross-provider match key.

### Login resolution algorithm

Runs at the seam, on every request bearing a token:

1. Verify token → connection type + verified `sub` / email / phone.
2. Compute the `Identity` key → look it up.
3. **Found** → `person_id` → `Person` → `Player` for `CURRENT_TOURNAMENT_ID` →
   `CurrentPlayer::Player`.
4. **Not found**, but the token's verified email matches another
   `Identity`/`Person` → AUTH-13 link path (§6).
5. **Not found**, but a **pending `Person`** exists for this verified email
   (or the request carries a valid invite code, §5) → claim path (§5).
6. **Not found**, no match anywhere → `CurrentPlayer::AuthenticatedUnclaimed`
   (AUTH-06).

The **result-user** is unchanged — a `Player` with an ordinary `Identity` (an
organizer-controlled email), seeded by `xtask seed`. "Admin" = able to log in
as it (ADMIN-04). No new concept.

## §4 — Login & session flows

- **Login surface.** A site-wide "Log in" action; per BROWSE-05, a visitor
  reaching My Tips / All Tips is bounced to it. The surface offers email,
  phone, and Google.
- **Production: Auth0 Universal Login (redirect).** The SPA uses the Auth0
  SPA SDK and calls `loginWithRedirect()`. Auth0's hosted page handles all
  three connections. Universal Login is Auth0's recommended, lowest-code,
  most secure path; customizing / i18n-polishing the hosted page is
  explicitly a later-phase concern (this design is functionality-only).
- **Dev / local: no redirect.** In `dev_auth` mode the SPA skips Auth0
  entirely — it shows a seeded-player picker, calls
  `POST /api/dev/login`, and gets a local-issuer JWT (§2). Same
  token-handling code path as production; only the token source differs.
- **Token handling.** The SPA SDK acquires and holds the JWT (in-memory by
  default — XSS-safer) and silently refreshes it. Every `/api/graphql`
  request carries `Authorization: Bearer <jwt>`. The API just validates per
  request (§2); it holds no session state.
- **Post-login routing:**
  - Resolves to a `Player` → lands on their destination (or Profile on first
    claim, §5).
  - Resolves to `AuthenticatedUnclaimed` → the SPA renders the AUTH-06 "you
    need an invitation to play" state; player-only routes stay refused.
- **Logout (AUTH-15).** SPA SDK `logout()` ends the Auth0 session and
  discards the token; user returns as a `Visitor`.

### Auth0 origins (resolved)

Per `infrastructure/env/*.tfvars`, one Auth0 tenant + one SPA application,
with all three origins registered as **Allowed Callback URLs**, **Allowed
Logout URLs**, and **Allowed Web Origins**:

| Env | Origin |
|---|---|
| prod | `https://pool.xczimi.com` |
| dev (cloud) | `https://pool-dev.xczimi.com` |
| local | `http://localhost:5173` |

Local dev does not actually hit Auth0 (the local-issuer JWT path handles that),
but registering `localhost:5173` keeps the door open for exercising the real
Auth0 flow locally if needed. Hobby scale — no reason to fragment into
multiple tenants or applications.

## §5 — Invite, claim & join flows

**An invite is a shareable link carrying a signed code — not an email.**

- A player opens **Invite** and gets a copyable link:
  `https://<origin>/invite/<code>`. The signed `code` payload carries
  `{ referrer: <playerId>, pool?: <poolId>, expiry, use-policy }`. They
  paste it into whatever chat they use. Use-policy varies by code kind:
  referral links are **single-use** (one claim consumes them); pool join
  links are **multi-use until rotated** (POOL-03), since the whole point is
  to admit many friends.
- App-sent email is **optional**: if the inviter also types an address, the
  app emails the same link via SES — convenience, not the mechanism. Most
  invites never touch SES.
- **Pool join links (POOL-02) use the same mechanism** — a code whose
  payload is pool-bound. One route, one code format; "referral" vs "pool
  join" is just whether `pool` is set. POOL-03's rotation/expiry applies to
  both.

### Lazy `Player` creation

No ghost `Person` / `Player` at invite time. The code holds the `referrer`;
nothing is created until someone claims. This **changes** AUTH-07's
"eagerly created" and **simplifies** AUTH-10 (there are no pending players to
hide because there are none until claim).

### Security

The invite link is a *referral grant*, not authentication. Opening it never
logs anyone in. The invitee still authenticates passwordless with their own
verified email/phone. A leaked link only lets a stranger become referred-by-you
/ join your pool — bounded, the code is expiring + rotatable, and there is no
impersonation risk.

### Claim flow

1. Invitee opens the link → SPA stashes the `code` → shows "log in to join"
   (passwordless / Google).
2. Invitee authenticates — verifies their own contact.
3. Seam: verified identity + stashed code → `claimInvite`:
   - Verified email already belongs to a `Person` → **AUTH-12** path: no
     new `Player`, just added to the pool (if any). AUTH-08's "already in the
     system" becomes a claim-time outcome, not an invite-time rejection.
   - Otherwise → create `Person` + `Player`, `referrer` from the code, join
     pool if set, prompt for **nick + full name** → Profile.

### AUTH-07 and AUTH-11 collapse

The invitee always supplies their own profile; the inviter types nothing
required (optionally an email to also-send, or a private label). The
`.specs/SCENARIOS.md` asymmetry between AUTH-07 (inviter types friend's
profile) and AUTH-11 (joiner supplies own profile) disappears.

## §6 — Linking a second identity (AUTH-13)

The principle is set in §3: the *first* provider attaches freely when an
account is claimed; **adding a second provider to an already-claimed `Person`
requires explicit confirmation — never silent** (legacy linked silently by
email; the rewrite rejects that).

- **Trigger.** §3 resolution step 4 — someone logs in with a new credential
  whose verified email matches an existing `Person` via a different
  provider. (Example: a `Person` who has only Google logs in via passwordless
  email to the same address.)
- **Prompt.** "An account already exists for `<email>`, signed in via
  `<other provider>`. Link this login to it?" — the `Identity` rows are
  linked only on explicit confirmation.
- **Confirm** → the new `Identity` row is created against the existing
  `Person`; that `Person` now has two identities, either works in future.
- **Decline** → no link, no duplicate. A second `Person` is **never** created
  for an email already owned. The session stays `AuthenticatedUnclaimed`;
  the user can re-enter with their original provider.
- Linking only ever joins `Identity` rows under one `Person`. `Player`
  records and predictions are untouched (identity is global, `Player` is
  per-tournament).

## §7 — Existing-spec corrections this design forces

These edits get applied to the authoritative `.specs/` during implementation:

### `.specs/DATA_MODEL.md`

- **§12 "Open / deferred"** — remove the "Auth mechanism — deferred (Auth0
  vs app-managed)" bullet. The mechanism is decided: Auth0, fully
  passwordless.
- **§3 entity table** — the Identity row's example ("Google sub,
  email+password, magic-link") becomes "Google sub, passwordless email,
  passwordless phone."

### `.specs/API.md`

- **§8 "Auth in the contract"** — rewrite. Drop "Phase 1 uses a dev stub."
  Describe the Bearer-JWT multi-issuer seam, three-state `CurrentPlayer`,
  in-app validation behind the `cloudfront_auth` middleware.

### `.specs/SCENARIOS.md`

- **"Design decisions baked into this catalog"** — Auth bullet drops
  "email+password"; adds "passwordless SMS (OTP code via Twilio)."
- **AUTH-01 / AUTH-02** — reworded. The dev mechanism is a local-issuer JWT
  (and a dev-login endpoint), not an `X-Dev-Player` header. Test names
  like `me_returns_player_when_authenticated` stay valid.
- **AUTH-05** — `keep` → `dropped`. Rationale: passwordless removes the
  motivation; no plaintext-or-otherwise password is stored anywhere.
- **New: AUTH-18 "Login via passwordless SMS"** — `future`. Auth0 sends a
  6-digit code via Twilio; user types it; resolves
  `Identity#phone#<E.164> → Person → Player`.
- **AUTH-07** — `changed` further. Eager → lazy; emailed branded message →
  shareable signed link (email optional); inviter no longer types friend's
  email / profile.
- **AUTH-08** — invite-time rejection → claim-time outcome.
- **AUTH-10** — simplified. Lazy creation means nothing pending to hide
  (unless the inviter explicitly pre-typed an email — even then, scoped to
  that path).
- **AUTH-11** — folded into AUTH-07's unified mechanism. Kept as the
  pool-join variant; the asymmetry note disappears.
- **AUTH-09** — `changed` further. The "Given a pending `Person`/`Player`
  exists" precondition is wrong under lazy creation; reword to
  "Given a valid invite code." Claim still happens on first passwordless
  login with the verified contact + stashed code, but the `Person`/`Player`
  is **created at claim time** rather than activated. Lands on Profile.
- **AUTH-03, 04, 06, 12, 13, 14, 15, 16, 17** — stand as written; minor
  rewording where they name the mechanism (e.g. AUTH-15's "Auth0 sessions"
  stays; AUTH-03/04's "Auth0 verifies" applies to any passwordless
  connection).

## Out of scope (later-phase)

- **Login UI / UX polish.** This design pins down the *functional* flows;
  Auth0 Universal Login carries production until a UI phase customizes it.
- **i18n of the Auth0 hosted page.** Auth0 supports it; the polish is a
  later-phase concern.
- **Account deletion / GDPR.** No requirement surfaced.

## Acceptance criteria

Implementation is complete when:

1. `cargo run -p api` with `LOCAL_AUTH_ISSUER` set (and `CLOUDFRONT_SECRET` /
   `AUTH0_DOMAIN` unset) serves a working local stack via the dev-login
   endpoint + local-issuer JWT. e2e's `global-setup.ts` uses this path and
   stays green.
2. With Auth0 env vars set, the seam validates Auth0-issued JWTs against the
   Auth0 JWKS and rejects local-issuer tokens (and vice-versa with
   `LOCAL_AUTH_ISSUER` set without Auth0).
3. All three Auth0 connections (passwordless email via SES, passwordless SMS
   via Twilio, Google) successfully log in to `pool-dev.xczimi.com` and
   resolve to a `Player`.
4. Invite link flow end-to-end: a logged-in player generates a copyable
   link; an unrelated browser session opens it, logs in passwordless,
   becomes a `Player` with `referrer` set, lands on Profile.
5. Pool-join link reuses the same code mechanism; AUTH-12 path (existing
   `Person`) joins the pool without creating a duplicate `Player`.
6. AUTH-13 explicit-confirmation linking prompts on a cross-provider
   email match and never silently links.
7. The `.specs/` corrections in §7 are applied in the same commit series.
