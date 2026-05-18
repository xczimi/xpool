# 14 — standings.ts mutates TeamStats objects in place

Status: done
Severity: HIGH
Area: web

## Problem

`computeStandings` (`web/src/lib/standings.ts:56-77`) retrieves `TeamStats`
objects from a `Map` and mutates them in place (`h.played += 1`,
`h.points += 3`, `a.goalsFor += score.away`, …).

This contradicts the project's mandated immutability rule ("ALWAYS create new
objects, NEVER mutate"). The objects are freshly created in local scope so it
is not a correctness bug today, but it violates the house style and is a trap
if the map is ever shared.

## Expected

Fold match results into new `TeamStats` objects (e.g. immutable update via
spread, or `reduce` over the fixtures).

## Acceptance

- No in-place mutation in `standings.ts`.
- Behaviour unchanged — covered by the tests added in issue 13.

## Comments

Rewrote `computeStandings` to fold games via `reduce` into a fresh `Map` of
fresh `TeamStats` objects — `emptyStats` + `recordMatch` build new objects with
spreads, no in-place mutation. Behaviour is unchanged and verified by the
issue-13 standings tests (including new "does not mutate the input" cases).
