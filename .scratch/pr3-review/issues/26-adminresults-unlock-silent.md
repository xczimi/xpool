# 26 — AdminResults unlock succeeds with no confirmation

Status: ready-for-agent
Severity: HIGH
Area: web

## Problem

The i18n string `resultUnlocked` ("Result unlocked — you can re-enter it.")
was added to both catalogues (`web/src/i18n/strings.ts:175,323`) but is never
referenced. `AdminResults.tsx` `onUnlock` calls `unlockResult`, refreshes, and
shows no confirmation — unlike the recompute path, which surfaces
`recomputeDone`. The unlock action succeeds silently.

## Expected

After a successful `unlockResult`, show the `resultUnlocked` notice (same
mechanism the recompute path uses for `recomputeDone`). Either wire the string
or, if a notice is deliberately not wanted, remove the orphaned key.

## Acceptance

- Unlocking a result shows a visible confirmation.
- `web` build + lint green; no orphaned i18n key.
