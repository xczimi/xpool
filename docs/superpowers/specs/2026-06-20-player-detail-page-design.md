# Player-detail page — design

Status: design approved (brainstorm 2026-06-20)
Consumer **#3** of the TheSportsDB feature decomposition
(`2026-06-14-sportsdb-reported-results-design.md` §1, row 3). PRD:
`.scratch/player-detail-page/PRD.md`.

## Summary

A read-only page per **pool participant** (a human predictor — *not* a
footballer) giving a complete view of that one player: their totals + rank,
per-round score breakdown, perfect tips, and — on demand — every match
prediction vs the official result, plus group-stage standings predictions.

It is a **vertical slice** of pages that already exist (Scoreboard's per-round
breakdown, All Tips' prediction grid, Standings badges, Perfects), filtered to
one participant and assembled under one route. It **does not touch SportsDB** —
pure frontend aggregation over xpool's existing GraphQL. It is therefore
independent of consumer #2 (live preview) and can be built in parallel.

## Route

`/player/:id` — `id` is a participant's player id.

- **Own page** (`id === me.id`): fully open, shows everything.
- **Another player's page**: pool-mate-gated (see Visibility).

## Page spine — summary-first, drill-down

A dense, always-visible header summarising the player's tournament, with a
collapsed per-round list beneath it that the viewer expands on demand. Default
state: **all rounds collapsed** — the page opens as the header plus a compact
list of round score-rows; every match detail is a deliberate tap.

## Component tree

Small, focused files (high cohesion, low coupling):

- **`PlayerPage.tsx`** — route shell. Reads `:id`, resolves the viewing
  context, loads the two header queries, derives the player's entry, renders
  header + rounds. Owns the page-level states (loading / error / not-found /
  not-a-pool-mate / no-predictions-yet).
- **`PlayerHeader.tsx`** — the dense top block (total + rank, per-round strip,
  perfects).
- **`PlayerRounds.tsx`** — the collapsed round list + expansion state.
- **`PlayerRoundDetail.tsx`** — one expanded round's content; owns its own lazy
  fetch and inline loading/error.

Reused as-is: `Matchup` (`TeamLabel`), `PointsBadge`, `StandingsBadge`,
`GroupSubNav`, `StatusViews` (`Loading`/`ErrorView`/`NeedsLogin`), and the
`lib/rounds.ts` helpers (`visibleRoundNodes`, `leafGroupsOfRound`,
`currentRoundNode`, `ROUND_ORDER`, `roundLabel`).

## Header (dense, always visible)

- **Total points + rank** — rank is **derived client-side** from the player's
  index in the already-ranked `scoreboard(pool)` list. No new GraphQL field.
- **Per-round strip** — the player's `StageScore[]` rendered as one compact
  horizontal strip (Group Stage · R16 · QF · SF · Final), each cell showing
  that round's points. Only `readyRounds` are shown (a knockout round whose
  bracket has not resolved is hidden, mirroring Scoreboard / All Tips). These
  per-round figures double as the score labels on the collapsed rows below.
- **Perfects** — a count badge **and** the detail list (which matches they
  nailed, with `Matchup` + `PointsBadge`), from `perfects` filtered to this
  player.

Explicitly **not** in the header: a separate standings-bonus subtotal (its
points already fold into the per-round figures; the standings *predictions*
themselves live in the Group Stage drill-down).

## Drill-down (collapsed by default)

One **collapsed row per round**, showing the round label + that round's score.
Tapping expands; `PlayerRoundDetail` then lazily fetches its data (nothing is
fetched for a round until it is opened — a natural fit for the all-collapsed
default).

- **Knockout round** → one `tips(roundNodeId)` query (the resolver walks the
  round node's recursive subtree, exactly as All Tips does for knockouts) →
  every match in the round as a row: `Matchup` + the player's prediction + the
  official result + `PointsBadge`.
- **Group Stage** → a `GroupSubNav` (reused) defaulting to the first group.
  Selecting a group lazily fires **both** `tips(groupId)` and
  `standings(groupId)` → that group's matches **plus** the player's group-table
  standings prediction (`StandingsBadge` + bonus), interleaved into the same
  section. One group at a time keeps each fetch small rather than loading all
  groups of the round at once.

## Data — Approach A (client-side reuse, lazy per round)

No backend change. The page composes existing resolvers
(`crates/api/src/gql/query.rs`):

- **Header**: `scoreboard(effectivePool)` + `perfects`, filtered client-side to
  `:id`. `effectivePool` defaults to the viewer's first pool, reusing the exact
  selector logic in `ScoreboardPage` (`undefined` → first pool; the page does
  not expose the pool picker itself — it inherits the viewer's default pool
  context).
- **Drill-down**: the existing `tips(groupId)` / `tips(roundNodeId)` and
  `standings(groupId)` queries, fired lazily on expand and filtered client-side
  to `:id`.

Rationale (chosen over a new `player(id)` resolver): adds no backend; **reuses
the single place tip-visibility gating is enforced** (the `tips` resolver), so
the rule cannot drift; and its lazy fetch maps cleanly onto the all-collapsed
default instead of eagerly aggregating everything.

## Visibility

Two layers, deliberately different in strength:

- **Page-level gate (soft).** If `:id` is neither the viewer nor present in the
  viewer's pool scoreboard, render a "not in your pool" empty state instead of
  the page. This is a soft UX funnel, consistent with the invite-auth soft-funnel
  posture — not a hard security boundary.
- **Pick-level gate (hard).** Drill-down predictions come straight from the
  `tips` / `standings` resolvers, which already enforce mutual-commitment
  gating: you see another player's pick for a match only once **both** of you
  have effective-locked it (or the match has opened). Un-revealable picks render
  as **locked placeholder cells** (the count of hidden picks stays visible). No
  gating logic is reimplemented on the client.

Your own page (`id === me.id`) shows everything, ungated.

## Entry points

A player's name/nick links to `/player/:id` from:

- **Scoreboard rows** (primary entry point).
- **All Tips** player-name column headers.
- **Perfects** list nicks.
- **Your own page** from the nav bar / profile-settings menu.

## Error & empty states

- Page-level (in `PlayerPage`): loading (header skeleton), `ErrorView` on query
  error, unknown/not-found `:id`, not-a-pool-mate gate, and a no-predictions-yet
  state.
- Per-round (in `PlayerRoundDetail`): its own inline loading and error, so one
  failed round's fetch does not blank the whole page.
- Visitor (not logged in): the page needs a viewer to resolve pool context and
  own-vs-other → `NeedsLogin` for un-authenticated access.

## i18n

New strings in `web/src/i18n/strings.ts` (English + Hungarian): page title,
"not in your pool", "no predictions yet", perfects count label, locked-pick
placeholder. Round labels reuse the existing `roundLabel` strings.

## Testing

Per the frontend-work-needs-e2e rule, the load-bearing behaviour is covered by
Playwright e2e (boots the full live stack):

- Own page shows everything (all rounds expandable, all picks visible).
- A pool-mate's page hides un-revealable picks as locked placeholders, and
  reveals them after mutual effective-lock.
- A non-pool-mate `:id` hits the soft page-level gate.
- Each entry point (scoreboard row, All-Tips header, Perfects nick, own-page
  nav link) navigates to the right `/player/:id`.
- All-collapsed on load; expanding a round lazily loads only that round.

Component-level (vitest): rank derivation from scoreboard index, per-round strip
rendering from `StageScore[]`, perfects filtering, lazy-expand fetch trigger.

## Out of scope

- SportsDB / live data (consumer #2).
- Editing predictions (read-only page).
- Cross-pool privacy changes; any new backend resolver.
- A pool picker on the page (inherits the viewer's default pool context).
