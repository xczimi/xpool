# 29 — useMemo dependencies are the whole `tournament` object

Status: done
Severity: MEDIUM
Area: web

## Problem

`MyTipsPage.tsx` / `AllTipsPage.tsx` memoize `roundNodes(...)` with `[tournament]`
as the dependency, and `AdminResults.tsx` memoizes `teamIndex(...)` likewise.
`roundNodes` depends only on `tournament?.groups` and `teamIndex` only on
`tournament?.teams`. urql returns a new `tournament` object identity on every
poll result even when the data is cache-identical, so these memos recompute on
every poll tick.

## Expected

Narrow each `useMemo` dependency to the field it actually reads
(`tournament?.groups`, `tournament?.teams`).

## Acceptance

- The memo deps reference only the consumed fields.
- `web` build + lint green.

## Comments

Narrowed the memo deps: `roundNodes` memos in MyTips/AllTips now depend on
`tournament?.groups`, and the `teamIndex` memo in `AdminResults.tsx` on
`tournament?.teams`. They no longer recompute on every urql poll tick.
