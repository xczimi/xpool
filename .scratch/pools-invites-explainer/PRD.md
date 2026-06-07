# Explain pools & invites to users

Status: needs-triage
Area: web / content

## Idea

Add clearer explanatory content about how **pools** and **invites** work — what
a pool is, how you join one, how inviting others works, and what's
private/shared.

## Motivation

Pools and invites are core concepts but easy to misunderstand: Is there one big
pool or many? How do I get someone in? Who can see my tips? The
`PoolsPage` / `InvitePage` / `InviteClaimPage` exist, but the *mental model*
isn't spelled out anywhere. Friendly explanation reduces "how does this work?"
friction and support questions.

## Sketch

- Short explainer copy on `PoolsPage` (what a pool is, joining, membership) and
  on `InvitePage` / `InviteClaimPage` (how an invite is created, sent, claimed,
  and what the recipient gets).
- Cover the privacy angle in plain words: who can see whose predictions, when
  (e.g. after deadline), and pool membership visibility.
- All i18n'd (EN + HU) in `web/src/i18n/strings.ts`.

## Open questions

- Does this belong inline on each page, in the Rules page, or a small "Help"
  section?
- How much overlaps with [[rules-content]] vs. stays page-local?

## Related

- [[invite-only-hardening]] — the invite mechanics being explained here are the
  same ones that PRD tightens; keep the messaging consistent.
- [[rules-content]] / [[page-one-liner-intros]] — sibling content work.
