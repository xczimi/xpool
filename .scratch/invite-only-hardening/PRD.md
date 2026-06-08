# Invite-only hardening — stop random Auth0 signups

Status: in-progress (branch `pools-invites`)
Area: web (funnel UI) + Auth0 tenant config (+ docs)

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
- **Bookmarkable pool/invite.** `invite/:code` route already exists; a
  pool-scoped link is the same signed code with `pool` set
  (multi-use-until-rotated already supported). Landing-experience detail folds
  into the explainer/simplification proposal ([[pools-invites-explainer]]).
- **Sessions ("don't log in too often") — Auth0 tenant config, documented:**
  Refresh Token **Absolute Lifetime = 90 days**, **Inactivity/Idle = 30 days**,
  rotation on. SPA already uses rotating refresh tokens.
- **i18n:** new funnel + dead-end copy in **EN + HU now**; leave the old
  prod-auth-bar English deferral alone (out of scope).
- **Hard gate (documented fallback, NOT built):** thread the invite code through
  the Auth0 `/authorize` request and validate its HMAC signature in an Auth0
  Action that denies login/registration without a valid code. Pull this only if
  junk signups actually materialise.

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

- Gate account creation / pool access on an invitation (the `invite` flow
  already exists in the API) — a self-registered Auth0 identity with no
  invitation should not become a usable player.
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
- **The result-user cannot own a pool** (POOL-12). So the admin can mint
  *referral* invites (`createInvite(pool: None)`) and `invite`, but the **first
  invited human** is who creates the pool — that's the bootstrap path.

## Related

- Pairs with [[dev-deploy-clock-and-auth]] — the dev deployment still needs a
  no-Auth0 path even as prod tightens.
