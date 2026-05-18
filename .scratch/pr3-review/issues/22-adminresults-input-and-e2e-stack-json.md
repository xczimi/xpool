# 22 — AdminResults score input unvalidated; e2e-stack.sh parses JSON with node

Status: done
Severity: MEDIUM
Area: web

Two small, independent web-tooling fixes bundled together.

## Problem A — AdminResults score input has no validation

`AdminResults.tsx:120` calls `Number(home)` / `Number(away)` on free-text
`<input type="number">` values with only an empty-string guard. A non-numeric
or negative value yields `NaN` / negative and is sent straight to the
`enterResult` mutation. `min={0}` is browser-advisory only. (`GroupTipForm`
mitigates this with a `<select>` of 0–9 — `AdminResults` should do likewise or
validate explicitly.)

## Problem B — e2e-stack.sh parses cargo metadata with inline node

`web/scripts/e2e-stack.sh:104-105` resolves the target dir by piping
`cargo metadata` JSON through an inline `node -e` script — brittle and depends
on `node` on PATH. The project preference is `jq`:
`cargo metadata --format-version 1 | jq -r .target_directory`.

## Acceptance

- AdminResults rejects / constrains invalid score input (matches GroupTipForm).
- `e2e-stack.sh` uses `jq`; e2e suite still boots.

## Comments

A: `AdminResults` now uses a constrained 0–9 `<select>` (`ScoreSelect`)
mirroring `GroupTipForm`, plus an `isValidScore` guard that disables Save /
Enter and blocks the mutation on out-of-range input. B: `e2e-stack.sh` now
resolves the target dir with `cargo metadata … | jq -r .target_directory`
instead of inline `node`.
