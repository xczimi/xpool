# Pools & invites — design explainer + simplification (for the maintainer)

Status: **shipped** (2026-06-08, branch `pools-invites`) — see `DESIGN.md` / `PLAN.md`
Area: docs / design understanding → simplification of crates/api auth + invite/pool

## Reframe (2026-06-07)

This was originally filed as *end-user* explanatory copy. Corrected intent: it's
**for the maintainer** — understand how the current pools / invites / auth system
is actually designed and implemented, then **simplify** it. Not website text.

(User-facing explanatory copy still happens, but as part of
[[invite-only-hardening]]'s funnel surfaces — front door + dead-end card — not
here.)

## Deliverable

1. **Visual explainer** (self-contained HTML) mapping the current design:
   - the signed HMAC invite code (`crates/api/src/auth/invite_code.rs`) —
     payload, `UsePolicy` (SingleUse vs MultiUseUntilRotated), expiry, signature
   - the three-state `CurrentPlayer` (`Visitor` / `AuthenticatedUnclaimed` /
     `Player`) and the §3 login-resolution algorithm (`auth/resolution.rs`)
   - the claim / link-second-identity / pool-join flows (`gql/mutation.rs`)
   - where **Auth0 vs app vs Dynamo** each sit (Auth0 mints identities; the app
     resolves + lazily creates Person/Player on claim; Dynamo only written on
     claim/link)
2. **Simplification proposal** — concrete opportunities that look over-built for
   a hobby pool, each with a tradeoff. No code changed until approved.

## Then

Once understood + approved → **do the agreed simplifications** on this branch,
then [[invite-only-hardening]] builds its funnel on the simplified design.

## Known design facts (already traced)

- **Dynamo is already safe** from uninvited signups — lazy `Player` creation
  means an uninvited login writes zero rows; only `claim_invite` / link write.
- **Bookmarkable invite links already exist** — `invite/:code` →
  `InviteClaimPage`. A pool-scoped link is the same code with `pool` set.
- **CLAUDE.md is stale on auth** — it still says "Auth is a dev stub …
  `X-Dev-Player` header; no real auth yet (deferred)", but a full multi-issuer
  Auth0 + local-issuer seam is implemented (`crates/api/src/auth/`). Flag for
  correction.
- Source design spec: `docs/superpowers/specs/2026-05-30-auth-design.md` (§3
  resolution, §5 invite/claim/join, §6 identity linking).
