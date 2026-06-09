# Identity-aware Home / welcome page — design

**Date:** 2026-06-09
**Status:** approved
**Builds on:** the merged pools/invite work
(`.scratch/merge-pools-invite-pages/PRD.md`) — reuses its `/invite` →
`NeedsInvite` code-entry behaviour.

## Goal

Let a non-player landing on Home **enter an invite code there**, with the same
effect as opening an `/invite/:code` link. More broadly, make the Home page
**identity-aware**: it shows each visitor what to do next based on their
`CurrentPlayer` state, instead of one static welcome + link list for everyone.

The original ask ("non-authenticated visitors should be able to enter an invite
code on Home") generalised, during brainstorming, into "show instructions on
the welcome page depending on the state of auth / invite / day." We scope **this
work to the identity dimension** (visitor / unclaimed / player) and **defer the
tournament-"day" dimension** (before-kickoff / running / finished) to a
follow-up.

## Background — what already exists

- Identity is the GraphQL `me` query's `__typename`: `Player` |
  `UnclaimedViewer`, or `null` for an unauthenticated viewer. Each page resolves
  this inline (`Layout`, `PoolsPage`, `ProfilePage`, …); there is **no** shared
  current-player hook, and this design does not add one (out of scope).
- `useAuth().label` is the client-side "a session exists" gate that unpauses the
  `me` query (a logged-out viewer has `label === null`).
- `NeedsInvite` already implements the recipient-side widget: a paste input →
  `extractCode()` (`components/inviteCode.ts`) → `navigate('/invite/:code')`. It
  bundles that widget with dead-end chrome (an invite-only explainer + a Log Out
  button). It is rendered both as the unclaimed-viewer dead-end (`Layout`) and as
  the public `/invite` route element.
- `HomePage` today: a static `homeWelcome` heading, `homeIntro`, and four public
  link tiles (Today, Scoreboard, Schedule, Rules).

## Design

### Component boundaries

- **`InviteCodeEntry`** (new, small, ~30 lines) — the paste-and-route widget in
  isolation: a text input, an Open button, a bad-code warning, calling
  `extractCode()` then `navigate('/invite/:code')`. **No auth coupling, no
  logout.** This is the unit that delivers "enter a code on Home → same effect
  as landing on `/invite/:code`."
  - *Depends on:* `extractCode`, `react-router` `useNavigate`, i18n.
  - *Used by:* `HomePage` (non-player branch) and `NeedsInvite`.
- **`NeedsInvite`** (refactor) — keeps its invite-only explainer and Log Out
  button, but renders `<InviteCodeEntry/>` in place of its own inline copy of the
  widget. Externally unchanged (same DOM contract the dead-end e2e relies on).
- **`HomePage`** (rewrite) — queries `me` (paused without a session, mirroring
  `Layout`/`PoolsPage`) and branches on identity.

This extraction is the one we deliberately deferred when `/invite` simply
rendered `NeedsInvite` whole. Home needs the widget **without** the explainer and
logout, so a shared `InviteCodeEntry` is now the clean boundary.

### Identity branches on Home

| State | How detected | Home content |
|---|---|---|
| **Visitor** | no session (`label` falsy) | welcome heading + invite-only explainer (`inviteOnly*` strings) + `<InviteCodeEntry/>` |
| **Unclaimed** | session, `me.__typename === 'UnclaimedViewer'` | same block as Visitor — one shared non-player block |
| **Player** | session, `me.__typename === 'Player'` | welcome heading + quick-action links: My Tips, Today, Scoreboard, Pools |
| **Loading** | session present, `me` in flight | neutral welcome heading + `homeIntro` only — no branch-specific block, to avoid a flash of the wrong state |

Visitor and Unclaimed render the **same** non-player block (both are "not in a
pool yet; paste your invite to get in"). The branch reads identity only; it does
**not** branch on tournament time (deferred).

### Data flow

`HomePage` → `useQuery(ME_QUERY, { pause: !label })`. `label` falsy ⇒ Visitor
branch, no query fired. Otherwise the resolved `me.__typename` selects Unclaimed
vs Player; `fetching && !data` ⇒ Loading branch. `InviteCodeEntry` owns its own
input state and navigation; Home passes it nothing.

### Error handling

- `InviteCodeEntry`: an unparseable entry (no code extracted) shows the existing
  `inviteOnlyBadLink` warning and does not navigate — same behaviour `NeedsInvite`
  has today.
- A logged-out viewer who pastes a code lands on `/invite/:code`, where
  `InviteClaimPage` prompts sign-in (unchanged). The `me` query returning `null`
  for an unauthenticated viewer is not an error.

### i18n

- Reuse existing `inviteOnly*` keys for the non-player block (title, body,
  have-link label, paste placeholder, Open, bad-link warning).
- Player branch reuses existing `nav*` labels for its links where possible; add
  new keys only if a label does not already exist.

## Testing

- **e2e (Home):** a logged-out visitor on Home sees the invite entry, and a
  pasted code/link routes to `/invite/:code`; a logged-in Player sees the
  quick-action links and **no** invite entry.
- **e2e (regression):** the unclaimed-viewer dead-end still renders
  `NeedsInvite` with its paste widget and routes correctly (the refactor must not
  change that DOM contract).
- **Unit:** `extractCode` coverage already exists (`inviteCode.test.ts`); no new
  unit test required for the pure helper.

## Deferred / follow-ups

- **Tournament-"day" dimension** on Home (before-kickoff → "enter your
  predictions"; running → "today's matches / deadlines"; finished → "final
  standings"). Separate idea/PRD.
- **D6** (from the merge work): guard `NeedsInvite`'s Log Out button on session
  presence for the public `/invite` page. Home sidesteps it (no logout in
  `InviteCodeEntry`), but `/invite` still renders full `NeedsInvite`.

## Out of scope

- A shared `useCurrentPlayer` hook (each page still resolves `me` inline).
- Any change to `/invite`, `/invite/:code`, or the Pools share/join surfaces.
