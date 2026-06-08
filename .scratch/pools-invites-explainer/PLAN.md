# Implementation plan — invite/referral/pool model

Status: **SHIPPED** (2026-06-08) · branch `pools-invites` · design in `DESIGN.md`

All phases landed bottom-up (storage → domain → api → web → i18n → docs → e2e), each
layer green before the next, TDD where practical. Verified: `cargo test --workspace`
(+ `DYNAMO_TEST=1` storage), `cargo clippy --workspace -- -D warnings`, `npm run build`,
`npm run lint`, `npm run e2e` (25/25).

One deviation from the bottom-up plan: the `join_code`/`claim_invite_code`/HMAC
**removals** were deferred from Phases 1–2 into Phase 3 (the API migration), so the
workspace stayed green at every commit rather than breaking mid-sequence.

## Phase 0 — Resolve the three open wrinkles (decisions, no code)
- **POOL-12 ownership:** relax so the admin can own pools, **or** designate a
  normal-player owner distinct from the results identity. Pick one.
- **Code format:** confirm ~10-char base32 (~50 bits); define alphabet (avoid
  ambiguous chars 0/O, 1/I/l).
- **Prefix handling:** mismatched pool prefix → validate + resolve by suffix (warn) vs
  hard error. Lean: resolve by suffix, ignore/validate prefix.
- ✅ Verify: decisions recorded back into `DESIGN.md`.

## Phase 1 — Storage: the invite table
- Add an `Invite` row type + `Repository` methods: `put_invite`, `get_invite(code)`,
  `list_invites(pool)`/`by_invited_by`, `revoke_invite`. Replace `claim_invite_code`
  (single-use marker) — reusable codes don't need it; revocation is the off-switch.
- Implement in `InMemoryRepository` (`crates/storage/src/memory.rs`) and
  `DynamoRepository` (`crates/storage/src/dynamo.rs`, key `INVITE#<suffix>`).
- Remove `Pool.join_code` plumbing from storage.
- ✅ Verify: `cargo test -p storage`; `DYNAMO_TEST=1 cargo test -p storage` for the
  Dynamo adapter.

## Phase 2 — Domain: Pool + referral semantics
- `crates/domain/src/model.rs` (**locked contract — additive/removal ripples
  everywhere; do this deliberately**): drop `Pool.join_code`. Keep `Player.referrer`.
- `crates/domain/src/pool.rs`: remove `set_join_code`; keep join/leave/rename/
  remove_member. Join stays the pure membership op.
- ✅ Verify: `cargo test -p domain`; workspace compiles.

## Phase 3 — API: collapse to one invite mechanism
- **Retire** `invite(invitee_id)` mutation and `crates/api/src/auth/invite_code.rs`
  (signed tokens, `INVITE_CODE_SECRET`).
- `create_invite(pool)`: mint a stored reusable invite row for the current member
  (`invited_by = me`), return `{ code, link }` with the nested displayed form.
- `join(code)` (was `claim_invite` minus the identity bits): resolve code → `{pool,
  invited_by}`, add the (already-identified) player to the pool, set
  `player.referrer = invited_by` if unset. Lenient resolution: full / suffix / bare
  prefix. Replace `join_pool` + `rotate_join_code` (→ `revoke_invite` / re-mint).
- Pool creation mints the owner's invite row (the pool link).
- Resolve clock-seam: expiry checked against the request `now(ctx)`.
- ✅ Verify: `cargo test -p api`; `cargo clippy --workspace -- -D warnings`.

## Phase 4 — Web: invite/join + default-to-pool view
- `InvitePage`: "share your invite" → the member's reusable nested link + copy.
- `InviteClaimPage` → join flow: lenient code entry box (paste link or type code).
- Default view = a pool the player is in (not the global board). Global board stays
  reachable but de-emphasised. Pool switcher for multi-pool members (you).
- Remove join-code UI on `PoolsPage`.
- ✅ Verify: `npm run build`; `npm run lint`.

## Phase 5 — i18n vocabulary cleanup
- `web/src/i18n/strings.ts` (EN + HU): adopt **invite / join / invited-by / sign-in**;
  purge user-facing "claim", "referrer", "join code". Keep the word "pool".
- ✅ Verify: build; visually check EN + HU.

## Phase 6 — Specs & docs
- Update `.specs/DATA_MODEL.md` (invite table, drop pool.join_code) and
  `.specs/SCENARIOS.md` (POOL-02/03 reframed onto invites).
- **Fix the stale CLAUDE.md auth note** ("dev stub / X-Dev-Player / no real auth").
- Flip `[[pools-invites-explainer]]` + `[[invite-only-hardening]]` PRDs to reflect
  what shipped.
- ✅ Verify: docs read consistently with code.

## Phase 7 — End-to-end
- E2E: create pool → member mints invite → second player joins via nested link →
  appears on that pool's board, not another's; `invited-by` recorded. Per repo memory,
  frontend work needs an E2E pass.
- ✅ Verify: `npm run e2e`.

## After this
[[invite-only-hardening]] soft-funnel (front door + dead-end view + 90d/30d sessions)
builds on the simplified model. Separate work, separate go-ahead.
