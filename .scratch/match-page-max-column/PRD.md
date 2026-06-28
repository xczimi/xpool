# Max-possible as its own column on the match page

Status: done — shipped via follow/match-page-polish, merged to master 2026-06-27
Area: web

## Idea

Promote the existing live "max still-achievable points" from an inline secondary
annotation (`Max ≤ N` rendered per row in `web/src/pages/MatchPage.tsx`) to a
proper **table column** in the match prediction list, sitting alongside the
what-if (±1 goal) columns added this round.

## Why now

The what-if columns ([[match-page-what-if-scores]]) just turned the match
prediction list into a live "what does the next goal do" view. Max-achievable
([[live-max-achievable-points]], shipped round 1) is the natural companion — the
ceiling each player can still reach for this match — and it already arrives from
the server as `row.maxReachable`. Making it a first-class column makes the live
race legible at a glance and lets you rank players by their ceiling.

## Resolved decisions (2026-06-27 grill)

- **Dedicated column, replacing the inline annotation.** Remove the inline
  `{t('maxReachableShort')} ≤ {row.maxReachable}` annotation; render a single
  clean Max cell per row under a column header. (`MatchPage.tsx:~230-235`.)
- **Sortable.** Add Max to the sortable headers built in
  [[match-page-sort-predictions]] (player / prediction / points / **max**), so
  players can be ranked by ceiling for this match. Reuse the existing
  `matchSort` helper + localStorage persistence.
- **Visibility:** keep the current gating — the column/value appears only when
  `row.maxReachable != null` (i.e. during the live window, same as today). No
  `Date.now()`; rely on the server-provided field.
- **Data:** no new resolver — `row.maxReachable` is already provided. Pure UI.

## Scope / file surface

- `web/src/pages/MatchPage.tsx` — move max-reachable into a column; add header;
  wire into the sort comparator.
- `web/src/lib/matchSort.ts` — extend the sortable key set with `max` (rows where
  `maxReachable == null` sink, consistent with the points column behaviour).
- `web/src/i18n/strings.ts` — column header key (EN + HU); reuse the existing
  `maxReachableShort` / `maxReachableTooltip` keys where possible.
- `web/src/index.css` — column styling consistent with the what-if columns.
- e2e: extend `web/e2e/match-page-sort.spec.ts` (or a small new spec) to assert
  the Max column renders during live and sorts.

## Sequencing

Follow-up to `cluster/match-page`; build on the `backlog-parallel-build` branch
AFTER the combined e2e is green (it touches `MatchPage.tsx` / `matchSort.ts`,
which the e2e-debugging pass may also touch — avoid the collision).
