# 13 — no unit/integration tests for web/src

Status: done
Severity: HIGH
Area: web

## Problem

`web/package.json` has no `test` script and there are zero `*.test.*` files
under `web/src`. Pure, branch-heavy logic — `standings.ts` (`computeStandings`,
`applyDrawOrder`, head-to-head / draw-order tiebreaks), `rounds.ts`,
`format.ts`, `polling.ts` — is only exercised indirectly via Playwright e2e.
The project's 80% coverage requirement is unmet for the SPA's logic layer.

## Expected

Add a unit-test runner (Vitest) and a `test` script; cover the pure logic
modules.

## Acceptance

- `npm run test` exists and runs.
- `web/src/lib/` pure functions covered (target 80%+).
- CI / `npm run` story documented.

## Comments

Added Vitest as the unit-test runner: `vitest.config.ts`, a `test` /
`test:watch` / `test:coverage` script in `web/package.json`, and `*.test.ts`
suites for `standings.ts`, `rounds.ts`, `format.ts`, `polling.ts`. Coverage of
`src/lib/` is 97% lines / 100% functions (the `usePolledQuery.ts` hook is
excluded — effect-driven, covered by e2e). The npm-run story is documented in
`web/README.md`.
