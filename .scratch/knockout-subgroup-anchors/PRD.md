# Hash anchors to jump into knockout sub-groups (no extra tab level)

Status: done — shipped via backlog-parallel-build (cluster/mytips-nav; link wiring in follow/match-page-polish), merged to master 2026-06-27
Area: web

## Resolved decisions (2026-06-27 grill)

- **IN, Wave 2** (`cluster/mytips-nav`) — unblocked by knockout-tip-labels shipping.
  Build this FIRST in the cluster, then [[mobile-prediction-entry]].
- **Anchor IDs:** use the stable `group.id` (already in the DB / GraphQL) as the element
  `id`; no volatile round-relative indices.
- **Routing:** the round tab stays the only routed level (`react-router` controls the
  path); the hash is client-side scroll behaviour and composes cleanly.
- **Scope:** cover BOTH group-stage and knockout sections (consistency).
- **Highlight-on-arrive:** optional brief highlight; smooth scroll is the core.
- **Files:** `web/src/pages/MyTipsPage.tsx` (hash `useEffect` + scroll),
  `web/src/pages/mytips/GroupTipForm.tsx` (add `id`), e2e deep-link test.

## Follow-up (2026-06-27, after review): wire the anchor into "Open this group"

**Gap found:** the anchors were built + tested but **no UI link generates the `#hash`**. The
three "Open this group" links (`MatchPage.tsx:184`, `TodayPage.tsx:124`, `SchedulePage.tsx:207`)
all point to bare `/mytips/<groupId>`. For group stage that isolates the one group (fine), but
for **knockout** the round **stacks every match**, so the link lands at the top of the round
instead of the specific match — exactly the case the anchor solves.

Decisions:
- **Append `#<group.id>`** to all three "Open this group" links so the target match scrolls
  into view (essential for knockout; harmless for group stage).
- **Knockout link text → "Open this KO match"** (derive group-vs-knockout from match arity,
  consistent with the knockout-tip-labels work). **Group-stage text stays "Open this group"**
  (no change). EN + HU.
- **Scope:** all three entry points (Match, Today, Schedule).
- Files: `web/src/pages/{MatchPage,TodayPage,SchedulePage}.tsx`, `web/src/i18n/strings.ts`,
  the link/label helper. Extend the anchor e2e to click an "Open this KO match" link and assert
  it scrolls to that match.
- **Sequenced AFTER the timeline rework** (same worktree); bundle with the what-if team-labels
  follow-up since both touch `MatchPage.tsx` + `strings.ts`.

## Idea

Use URL `#` anchors to jump directly into sub-sections of a knockout round,
the way group sections already work, **without** introducing a lower level of
tabs. Example: `https://pool.xczimi.com/mytips/R32#M76` to land on match M76
within the Round of 32, instead of a dedicated tab route like
`https://pool.xczimi.com/mytips/KO-M76`.

## Motivation

Knockout rounds have many matches; deep-linking to a specific one is useful
(sharing, the [[knockout-tip-labels]] work, reminders). But adding a second tier
of tabs for each knockout match would clutter the navigation. Hash anchors give
deep-linkability for free and mirror the existing group-section scrolling
behaviour, keeping the tab hierarchy flat.

## Sketch

- Render each knockout match section with an `id` (e.g. `M76`).
- On load with a `#M76` hash, scroll that section into view (and maybe briefly
  highlight it).
- Keep the round tab (`R32`) as the only routed level; the match is an anchor
  within it, not a route.
- Generate shareable `…/mytips/R32#M76` links from match references.

## Open questions (this idea is explicitly flagged for more discussion)

- Anchor IDs: stable match codes (`M76`) vs round-relative indices?
- How does this interact with the existing tab routing / `react-router` setup —
  does the round need to be the active tab before the anchor resolves?
- Should it also apply to group stage, or knockout-only?
- Highlight-on-arrive behaviour, and how it coexists with sticky pool/tab state.
- Relationship to [[knockout-tip-labels]] (current work) — does that change the
  section structure these anchors would target?
