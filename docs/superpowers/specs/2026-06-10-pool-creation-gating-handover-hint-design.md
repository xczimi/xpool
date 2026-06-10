# Pool-creation gating + solo-pool handover hint

**Date:** 2026-06-10 · **Branch:** `pool-creation-gating`

## Problem

Two pool-management UX gaps on `/pools`:

1. **Create-pool form shows for everyone.** Pool creation is restricted —
   `domain::pool::may_create_pool` allows it only for the result user and its
   direct referrals ("restricted creation, open inviting"). But the create form
   (`PoolsPage.tsx:152–166`) is always rendered; an unauthorized user fills it in,
   submits, and only then gets a backend error ("you are not allowed to create
   pools"). The frontend has no signal to hide the form: the `me` query exposes
   `isResultUser` but not `referrer` or any computed permission.

2. **Solo-pool handover is silently absent.** The handover dropdown renders only
   when a pool has a member other than the owner (`PoolsPage.tsx:299`), because
   `transfer_ownership` rejects a non-member recipient (`PoolError::NotAMember`).
   An owner whose pool is still just themselves sees *nothing* — which reads as a
   bug ("I can only hand over my first pool") rather than "there's no one to hand
   it to yet."

## Decisions (from brainstorming)

- **Handover target stays members-only.** No change to `transfer_ownership` or its
  member-only rule; no "transfer to any player" or "transfer link" flow. The
  member-only constraint is correct — the fix is legibility, not new capability.
- **Single source of truth for the permission.** The frontend must not re-derive
  `may_create_pool`; expose a computed boolean from the API that calls the existing
  domain function.

## Design

### Part 1 — gate the create-pool form

**Backend** (`crates/api`):

- Add `may_create_pool: bool` to the viewer `Player` GraphQL type
  (`gql/types.rs:250`, the `me` viewer object — not the pool-member type).
- Compute it in the `me` query root (`gql/query.rs:148`): load the result-user id
  once and call `domain::pool::may_create_pool(player, &ruid)`. The lookup lives in
  the query root (which already does I/O to resolve `me`); the resolver stays
  I/O-free and the rule stays in `domain` — the same function the `createPool`
  mutation enforces.

**Frontend** (`web`):

- Add `mayCreatePool` to `ME_QUERY` (`graphql/queries.ts:41`), Player variant.
- Gate the create-pool form (`PoolsPage.tsx:152–166`) on it: render the form only
  when `me.__typename === 'Player' && me.mayCreatePool`. Unauthorized players and
  unclaimed viewers don't see it.

### Part 2 — solo-pool handover hint

- In `PoolsPage.tsx:299`, the owner branch: when the pool has **no** member other
  than the owner, render a short disabled hint (e.g. *"Invite someone to hand this
  pool over."*) in place of the hidden dropdown. Pools with other members keep the
  existing dropdown unchanged.
- New i18n strings (en + hu) in `web/src/i18n/strings.ts` for the hint.

## Testing

- **Domain:** no new logic — `may_create_pool` and `transfer_ownership` are already
  covered (`crates/domain/tests/pool.rs`). No new domain tests.
- **API:** add a test asserting `me { mayCreatePool }` is `true` for an authorized
  player (result user or direct referral) and `false` for a non-referred player.
- **E2E (Playwright):** (a) authorized user sees the create form, non-authorized
  user does not; (b) a solo owned pool shows the hint, a multi-member owned pool
  shows the handover dropdown.

## Out of scope

- Any change to `transfer_ownership`, the member-only handover rule, or who may
  create pools.
- Exposing `referrer` or other Player internals to the frontend — only the computed
  `mayCreatePool` boolean is added.
