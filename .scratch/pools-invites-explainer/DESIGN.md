# Invite / referral / pool model — agreed design

Status: **shipped** (2026-06-08) · branch `pools-invites` · agreed (grilled 2026-06-07)
Supersedes the over-complicated framing in `~/.agent/diagrams/xpool-invites-explainer.html`.

Scope note: **identity / sign-in / Auth0 are out of scope here** (assume identity is
established). This is purely the invite → referral → pool → code model. The soft-funnel
hardening ([[invite-only-hardening]]) builds on top of this later.

## The model in plain words

1. **A pool is an invite-bound group** — friends, work, southsiders. Users don't
   "manage pools"; they get invited into one and that becomes their default view.
2. **One referral table.** Every invite is a row `{ code, pool, invited_by }`. The
   **pool link is just the owner's invite**; every member's invite is a sibling row —
   same pool, that member as `invited_by`. Pool link and invite link are *one
   mechanism*, differing only by which row's `invited_by` you look at.
3. **Reusable, per-member codes.** One code per member, per pool. Whoever uses Ada's
   code is "invited by Ada" into that pool. No per-email invite tracking — email is
   just a delivery helper for the same link.
4. **Codes are short, high-entropy, and forgiving to enter:**
   - The real key is a high-entropy personal code (e.g. `AD9X…`), globally unique.
   - The **displayed** link prepends a readable pool label → `SOUTH7K-AD9X`, so a human
     can see it belongs to Southsiders.
   - **Lenient entry:** full `SOUTH7K-AD9X`, bare suffix `AD9X`, or bare pool prefix
     `SOUTH7K` all resolve. The suffix is the unique key; the prefix is a cosmetic,
     validatable label. Bare prefix = the pool link = join "invited by the owner".
   - Works as a URL *or* a typed code. "Harder to guess" = more entropy than today's
     8-hex join code.
5. **Default pool, nobody poolless.** Joining via an invite puts you in that pool;
   your pool's board is your **default view**. The global all-players board stays in
   the API for future flexibility but is **not** the default user experience (keeps
   work/friends/southsiders from mixing in normal use).
6. **Restricted creation, open inviting.** *You* (admin) create the pools; *any
   member* can share their own invite into a pool they're in. Letting members create
   their own pools (permission granted by their referrer) is **parked** — additive
   later, not a foundation now.

## Data model

- **New: an invite/referral table.** Rows keyed by code, e.g. `INVITE#<suffix>`:
  `{ code, pool_id, invited_by (player id), created_at, expires_at?, revoked? }`.
  Reusable (multi-claim). The pool's canonical invite is the row with
  `invited_by = pool.owner`, minted at pool creation.
- **`Player.referrer` stays** — it's the *realized* edge (who invited THIS player),
  copied from the used code's `invited_by` at join time. The invite table holds the
  *channels*; `Player.referrer` holds the *realized referral graph*. (Reusable codes
  mean one row → many joiners, so the per-joiner edge must live on the player.)
- **`Pool.join_code` is removed** — replaced by the owner's invite row. `Pool` keeps
  `{ id, name, owner, members[] }`.

## What collapses (the simplification)

Three overlapping mechanisms → one stored invite table:

- **Retire `invite(invitee_id)`** (legacy dev-stub referral) — obsolete.
- **Retire signed HMAC tokens** (`crates/api/src/auth/invite_code.rs` encode/decode,
  `INVITE_CODE_SECRET`) — replaced by stored codes. A random high-entropy code can't
  be forged because it simply won't exist in the table; no signature needed. This also
  **kills the `Utc::now()` clock-seam** (expiry is a stored field, checked against the
  request clock).
- **Fold `pool.join_code` / `join_pool` / `rotate_join_code`** into the invite table
  (`rotate` → revoke/re-mint an invite row).

## Vocabulary cleanup (user-facing strings only; keep code identifiers)

| Now (messy) | User-facing word | Means |
|---|---|---|
| pool | **pool** (kept — rename to "circle" judged overkill this late) | a group you belong to & see a board for |
| join_code / invite code / referral | **invite** | one code = a referral into a pool |
| join_pool / claim_invite | **join** | accept an invite → become a member |
| referrer | **invited by** | who brought you in |
| login / auth / claim | **sign in** | establish who you are (out of scope here) |

`Pool` stays as the code/schema/spec identifier (locked contract). "Circle" is a
contained i18n-only flavour we can adopt anytime — not now.

## Decisions log (what was grilled, 2026-06-07)

- Q1 invite always tied to a pool; restricted pool creation → **yes**.
- Q2 creation admin-only, inviting open to any member → **yes**; referrer-granted
  creation permission **parked**.
- Q3/Q4 global board vs silo → **keep the global board in the API (b)** for future
  flexibility, but pools are the **default view** so normal use doesn't mix groups.
- Q5 code representation → **one stored short-code table** (drop signed HMAC + drop
  separate pool join_code).
- Q6 use policy → **reusable per-member** (one code per member per pool); no per-email
  tracking; email is a delivery helper.
- Q7 pool-link = owner's invite; member invites are sibling rows → **visible nesting**
  in the displayed URL, stored as independent unique codes, lenient entry.
- Q8 vocabulary → adopt **invite / join / invited-by / sign-in**; keep the word "pool".

## Resolved wrinkles (Phase 0 decisions, 2026-06-07)

- **POOL-12 ownership → keep POOL-12 intact; pool-creation is a referral-graph rule.**
  The result-user is **never** an owner/member (POOL-12 unchanged) and is the **root of
  the referral graph**. A player may create pools **iff `player.referrer ==
  result_user_id`** ("admins"). Admins are seeded with `referrer = result_user` (the
  one poolless, bootstrap-only referral edge); everyone else is invited *into a pool* by
  a normal member, so their referrer is a normal player and they **cannot** create
  pools. This realizes "restricted creation, open inviting" as a data rule, not a
  hardcoded admin list. `create_pool` gate changes from `!is_result_user` to
  `referrer == result_user` (and still rejects the result-user itself).
- **Code entropy / format → 10-char Crockford base32 (~50 bits).** Alphabet
  `0123456789ABCDEFGHJKMNPQRSTVWXYZ` (no I/L/O/U; no ambiguous 0/O 1/I/l). Suffix is the
  globally-unique key, generated from OS randomness. Entry is case-insensitive and
  tolerates dashes/whitespace.
- **Prefix handling → resolve by suffix; prefix is advisory.** Full `SOUTH7K-AD9XK3P7QT`,
  bare suffix `AD9XK3P7QT`, or bare prefix `SOUTH7K` all resolve. A mismatched prefix
  does **not** hard-error — resolve by suffix, surface a soft warning. The prefix is a
  **unique-per-pool** label (name slug + short disambiguator) so bare-prefix reliably
  maps to that pool's owner invite.

## Phase 3 clarification — invite is the front door to identity (2026-06-07)

The invite link is intrinsically the entry point to identity: clicking
`SOUTH7K-AD9X` pushes a logged-out visitor to log in / create an Auth0 user
(**establish identity**), then the invite **completes** (join pool, record
`invited_by`). An already-logged-in user skips straight to the simple join.

So identity-establishment is **not** ripped out of the invite flow — only the
real Auth0 mechanism is deferred. Today's lazy Player-creation is the **dev
stand-in** for "Auth0 just signed you up"; Auth0 later replaces that stand-in.
Identity *code* is fair game to restructure; only the *design doc* is
invite/pool-scoped.

Resulting Phase 3 shape (two entry points off the one stored table):
- **`join(code)`** — already-logged-in `Player` → resolve code (lenient) → add
  to pool, set `referrer = invited_by` if unset. The simple case.
- **`claim_invite(code, nick, fullName)`** — signed-in-but-no-Player-yet
  (`AuthenticatedUnclaimed`) → establish the Player (dev stand-in for Auth0
  signup), set `referrer = invited_by`, join. Re-pointed at the table; no HMAC.
- The HMAC token, `INVITE_CODE_SECRET`, single-use marker, `invite(invitee_id)`,
  `join_pool`, `rotate_join_code` are all retired; resolution is the stored row.

## Out of scope here

- The **real Auth0 sign-in/signup mechanism** (the dev lazy-create stand-in
  stays until Auth0 lands).
- The hard Auth0 signup gate (documented fallback in [[invite-only-hardening]]).
- Renaming `Pool` → `Circle` in code/specs.
