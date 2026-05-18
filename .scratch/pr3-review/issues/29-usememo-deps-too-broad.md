# 29 — useMemo dependencies are the whole `tournament` object

Status: ready-for-agent
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
