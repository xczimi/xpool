# Rules content — basics on the home page, full rules on the Rules page

Status: done (branch `rules-content`, 2026-06-09) — RulesPage moved off
hardcoded English into i18n'd EN+HU strings; Home gained a short i18n'd "how it
works" block (predict → earn → climb) linking to the full Rules page. Scoring
detail still sourced from `lib/rounds` `STAGE_MULTIPLIERS` so it can't drift.
e2e: `rules-content.spec.ts`. Open question "worked examples on Rules page"
deferred — prose only for now.
Area: web / content

## Idea

Put the **basic** rules (how the pool works, how scoring works at a glance) on
the home page, and write a fuller, detailed version on the dedicated Rules page.

## Motivation

A newcomer landing on `HomePage` should immediately get the gist — "predict
scores, earn points, climb the board" — without hunting. The `RulesPage`
(`rulesTitle: 'Rules & Scoring'`) should then carry the complete, authoritative
rules for anyone who wants the detail.

## Sketch

- **Home (`web/src/pages/HomePage.tsx`):** a short "how it works" block — a few
  lines / steps + the scoring summary (e.g. exact score / correct outcome
  points). Link through to the full Rules page.
- **Rules (`web/src/pages/RulesPage.tsx`):** the detailed rules — full scoring
  table, deadlines/locking, group vs knockout handling, tiebreaks, edge cases.
- Source the scoring detail from `.specs/SCORING.md` (the authoritative scoring
  doc) so the page can't drift from the engine.
- All copy i18n'd (EN + HU) in `web/src/i18n/strings.ts`; check
  `.specs/LEGACY_I18N.md` for prior wording. Note the legacy Euro-2008 rules
  copy was deliberately *not* carried over (strings.ts:10) — write fresh.

## Open questions

- How much scoring detail belongs on home vs. a "see full rules" link?
- Worked examples on the Rules page (a sample prediction → points), or prose?

## Related

- [[page-one-liner-intros]] — the home/rules one-liners are part of that pattern.
- [[pools-invites-explainer]] — pools/invites explanation may live in or beside
  the Rules page.
