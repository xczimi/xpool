# Merge the Pools and Invite pages

Status: needs-triage
Area: web

## Idea

Fold the standalone **Invite** page into the **Pools** page so there's a single
place to manage pools and share invites, instead of two pages that overlap.

## Motivation

The two pages already do the same thing. `InvitePage`'s own docstring says it
outright:

> Share your invite into a pool you belong to. ... **The same action lives
> per-pool on the Pools page; this is the standalone entry point.**

So invite creation exists twice:

- `PoolsPage` — per-pool invite/share controls inline with each pool the user
  owns or belongs to (`CREATE_INVITE_MUTATION`, plus revoke, members, etc.).
- `InvitePage` — a pool-selector dropdown + a single "share invite" action, a
  thin subset of what Pools already offers.

Two entry points for one concept (the invite is the front door to identity —
see [[invite-is-front-door-to-identity]]) means duplicated UI, duplicated i18n
strings, and a nav choice that doesn't map to a real distinction. A newcomer has
to guess why "Pools" and "Invite" are separate.

## Sketch

- Make **Pools** the single home for pool membership *and* invite sharing — the
  per-pool invite controls already live there.
- Remove the standalone `InvitePage` route + nav entry (or redirect it to Pools).
- Keep `InviteClaimPage` untouched — that's the *accept* flow (the recipient
  side), a genuinely different concern from creating/sharing.
- Reconcile i18n: drop now-orphaned `shareInvite`-style keys, or repoint them at
  the Pools surface.

## Open questions

- Is there a use case for sharing an invite *without* first seeing the pools
  list (e.g. a deep-link target)? If so, a redirect that pre-selects the pool
  may be better than a hard removal.
- Does anything (docs, the invite explainer, emails) link directly to
  `/invite`? Those references would need updating.

## Related

- [[invite-is-front-door-to-identity]]
- [[invite-auth-soft-funnel]]
- `.scratch/pools-invites-explainer/DESIGN.md` — the maintainer design doc for
  how pools + invites fit together.
- `.scratch/page-one-liner-intros/PRD.md` — lists both `PoolsPage` and
  `InvitePage`; merging removes one of them from that list.
