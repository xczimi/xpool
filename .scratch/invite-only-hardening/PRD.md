# Invite-only hardening — stop random Auth0 signups

Status: needs-triage
Area: api / auth (+ web)

## Idea

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
