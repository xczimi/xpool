# Hash anchors to jump into knockout sub-groups (no extra tab level)

Status: ready-for-agent — decisions resolved 2026-06-27 (Wave 2, cluster/mytips-nav)
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
