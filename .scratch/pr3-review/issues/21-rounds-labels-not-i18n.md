# 21 — rounds.ts round labels hardcoded in English, not i18n'd

Status: done
Severity: MEDIUM
Area: web

## Problem

`ROUND_LABELS` (`web/src/lib/rounds.ts:4-12`) is a plain
`Record<Round, string>` of English strings ("Group Stage", "Round of 32", …).
i18n is first-class (English + Hungarian) per CLAUDE.md; round names rendered
from this constant will not translate to Hungarian.

## Expected

Route round labels through the `web/src/i18n/strings.ts` catalogue.

## Acceptance

- Round names render in Hungarian when the locale is `hu`.
- Covered by the i18n / e2e tests.

## Comments

Replaced the hardcoded English `ROUND_LABELS` with i18n routing: added
`roundGroupStage`…`roundFinal` keys (en + hu) to `strings.ts`, and
`roundLabelKey` / `roundLabel(round, t)` helpers in `rounds.ts`. SchedulePage,
RulesPage and ScoreboardPage now render via `roundLabel(r, t)`, so round names
translate to Hungarian.
