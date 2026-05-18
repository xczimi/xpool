# 31 — All Tips knockout grid rebuilds derived data every render

Status: done
Severity: MEDIUM
Area: web

## Problem

`AllTipsPage.tsx` rebuilds `teams` (`teamIndex`), `tipMap`, and `players` on
every render — none are memoized, unlike `AdminResults.tsx`. For a knockout
round the grid is up to 16 match columns × N players, all rendered into the
DOM (the `.grid-scroll` wrapper only adds horizontal scroll). Fine at current
pool sizes; a perf cliff as pools grow.

## Expected

Memoize `teams`, `tipMap`, and `players` with appropriate dependencies. (No
virtualization needed yet — note it as a future concern only if pools grow
large.)

## Acceptance

- The derived maps are memoized.
- `web` build + lint + test green.

## Comments

Memoized `teams` (`[tournament?.teams]`), `tips` (`[tipsResult.data]`),
`players` and `tipMap` (`[tips]`) in `AllTipsPage.tsx`. Hoisted `tipKey` to
module scope so the `tipMap` memo can use it. Memos sit before the early
returns to keep hook order stable. Virtualization noted as a future concern.
