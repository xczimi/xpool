# Invite-only hardening — stop random Auth0 signups

Status: **shipped** (merged to master, 2026-06-08) — soft funnel built; Auth0
tenant settings + hard-gate fallback documented (not built). The Auth0 front
door goes live only once real Auth0 sign-in is enabled; the dead-end works in
any auth mode today. Builds on the **shipped** invite model
([[pools-invites-explainer]]).
Area: web (funnel UI) + Auth0 tenant config (+ docs)

## Implementation (2026-06-08)

Soft-funnel UI shipped on the branch:

- **Front door** — `ProdAuthBar` (Auth0 mode) shows an invite-oriented lead
  (`frontDoorLead`); "Members: log in" (`frontDoorMembers`) is secondary and
  passes `screen_hint: 'login'`. Localised EN + HU.
- **Dead-end** — new `NeedsInvite` content view (explainer + paste-your-link
  input → routes to the public `/invite/:code` claim page + log out). Replaces
  the old `UnclaimedBanner` (deleted). Shown by `Layout` when an
  `AuthenticatedUnclaimed` viewer (no link candidate) hits a non-public route.
- **Route gating** — shared `auth/routeAccess.ts` (`accessFor`) is the single
  source for `NavBar` (player/admin links hidden for a non-Player) and `Layout`
  (dead-end vs page). `/invite/:code` stays **public** — the way out.
- **Tests** — `inviteCode.test.ts` (unit, link/code extraction) + an
  `auth.spec.ts` e2e asserting the dead-end + public-page reachability via a
  planted unclaimed JWT.
- **Docs** — Auth0 session lifetimes (90d absolute / 30d idle, rotation on) and
  the hard-gate Auth0-Action fallback recorded in `.specs/DEPLOYMENT.md §9`.

Not built (documented fallbacks): the hard `/authorize` invite-code gate; Auth0
tenant session settings are dashboard config, not code.

## Decided design (2026-06-07, grilled with the maintainer)

**Soft-funnel only — usability over extreme security.** No hard Auth0 gate is
built; it's documented as a fallback. Rationale: Dynamo is already safe (lazy
`Player` creation — an uninvited login writes zero rows; only `claim_invite`
writes), so the only exposure is Auth0 identity quota, and the realistic threat
to an obscure hobby URL is confused humans, not bots. Free tier ≈ 25k MAU.

- **Front door (funnel-shaped).** For a code-less visitor the primary CTA is
  invite-oriented ("Got an invite? Open your link"); **"Members: log in"** is
  present but secondary. `loginWithRedirect` gets `screen_hint: 'login'` so
  returning members land on login, not a signup-flavoured screen.
  Replaces today's bare "You are outside / Log in" prod auth bar
  (`web/src/components/AuthBar.tsx` `ProdAuthBar`).
- **Dead-end (dedicated view).** `AuthenticatedUnclaimed` (no `linkCandidate`)
  renders a single "You need an invite" explainer card (short pools/invites
  explanation + "I have a link" action + log out) in the content area instead of
  the erroring page; player nav hidden; **Home/Rules stay public**. Upgrades
  today's thin `UnclaimedBanner`.
- **Bookmarkable pool/invite.** `invite/:code` route already exists; every invite
  is a **stored, reusable** pool-bound code (shipped). A bare pool prefix resolves
  to the owner's invite (the pool link). Landing-experience detail folds into the
  shipped explainer ([[pools-invites-explainer]]).
- **Sessions ("don't log in too often") — Auth0 tenant config, documented:**
  Refresh Token **Absolute Lifetime = 90 days**, **Inactivity/Idle = 30 days**,
  rotation on. SPA already uses rotating refresh tokens.
- **i18n:** new funnel + dead-end copy in **EN + HU now**; leave the old
  prod-auth-bar English deferral alone (out of scope).
- **Hard gate (documented fallback, NOT built):** thread the invite code through
  the Auth0 `/authorize` request and validate it in an Auth0 Action that denies
  login/registration without a code that **exists (and is unrevoked/unexpired) in
  the invite table** — a random code that isn't stored is unforgeable, so no
  signature is needed (the HMAC token was retired). Pull this only if junk signups
  actually materialise.

**Sequencing:** built on top of the (possibly simplified) design from
[[pools-invites-explainer]] — understand/simplify first, then this funnel.

## Idea (original)

Tighten the site to genuinely invitation-only so that strangers who stumble on
the URL can't fill up Auth0 with random accounts. Possibly add a private,
bookmarkable pool URL as the intended landing point.

## Motivation

With open Auth0 signup, anyone who finds the site can create an account. For a
small friendly pool that's noise at best and abuse at worst. Membership should
follow invitations, not public self-registration.

## Sketch

- Gate account creation / pool access on an invitation (the invite flow —
  `createInvite` / `join` / `claimInvite` — exists in the API) — a self-registered
  Auth0 identity with no invitation should not become a usable player.
- Consider a private pool URL (with a hard-to-guess token) people bookmark and
  land on, rather than a public front door.
- Decide what an uninvited visitor sees (a "request an invite" / dead-end page).

## Open questions

- Enforce at the Auth0 layer (disable open signup / use an allowlist) or in the
  app (accept the login but withhold player access until invited)?
- Should the private pool URL token be per-pool, single-use, or rotating?
- How are existing/legacy members migrated into the invite model?

## Note (2026-06-07) — bootstrap finding

Two facts established while debugging the dev deployment:

- **The admin identity must be linked to the result-user.** Logging in as
  `pool@xczimi.com` only resolves to the result-user/admin if an
  `IDENTITY#email#pool@xczimi.com` row exists (i.e. `RESULT_USER_EMAIL` is set to
  that email and the table is seeded). Otherwise the viewer is
  `AuthenticatedUnclaimed` and `CurrentPlayer::require` rejects every mutation.
- **The result-user cannot own a pool** (POOL-12), but it is the **referral-graph
  root**: players it referred directly are *admins* who may create pools
  (`may_create_pool`, shipped). Bootstrap is now seeding — admins get
  `referrer = result-user` (the seed makes all demo players founders); the
  poolless referral-only invite (`createInvite(pool: None)`) and the legacy
  `invite(inviteeId)` are **gone** (every invite is pool-bound).

## Update (2026-06-08) — result-user bootstraps pools in-app (supersedes above)

The "seeding-only" bootstrap above was a dead end on the deployed env: the only
login-reachable account there is the result-user (Auth0 at `pool@xczimi.com`),
and the seeded founders use unreachable `@dev.invalid` emails — so nobody could
create a pool or invite anyone. **POOL-12 is now revised** (SCENARIOS.md): the
result-user *may* create a pool as a **transient bootstrap owner** (owns but is
never a member), invite the first players (who become founders via the recorded
`invited_by` referrer), then **hand the pool over** to one of them
(`transferOwnership`, POOL-13) and detach. `may_create_pool` now allows the
result-user; `createPool` gives a result-user-owned pool an empty member list.

## Related

- Pairs with [[dev-deploy-clock-and-auth]] — the dev deployment still needs a
  no-Auth0 path even as prod tightens.
