# Knockout-aware tip labels

Status: ready-for-agent

Detailed, agreed requirements for a **frontend-only** copy/UX fix. Hand these to
a fresh session to write the design doc + implementation plan (run
`superpowers:brainstorming` → `writing-plans`, or go straight to a plan since the
design below is already settled with the user).

## Problem

Each knockout match is modelled as a **one-match "fake group"** with two teams
(`KO-M73` … `KO-M104`, `carries_standings: true`). The same prediction UI as the
12 real group-stage groups is reused, so a single knockout match is shown with
group-stage language that is wrong in that context:

- The reorder hint **"Drag tied teams to set your tiebreak order."** — in a
  knockout match the reorder isn't a generic "tiebreak"; it decides who advances
  on **extra time / penalties** when the 90-minute score is a draw.
- The heading **"Predicted standings" / "Actual standings"** — there's no
  standings table for a single match.
- The **"Pts"** column (league points) is meaningless for one match — flagged by
  the user as the "points" misnomer.

## Scope (confirmed)

- **Frontend SPA only.** No `crates/*`, no GraphQL schema, no query, no DB change.
  - The knockout flag `group.round` is already in the schema and already selected
    by the `Tournament` query (`web/src/graphql/queries.ts:8`).
  - The `drawOrder` submit path is unchanged — display-only change.
- Applies **only to knockout one-match (two-team) groups**, detected as
  `group.round !== 'GroupStage'`. The 12 group-stage groups keep today's wording,
  headings, and `P / GD / Pts` columns **unchanged**.
- Goes on a **branch/worktree** (web source per the working agreement), verified
  with an **e2e run** before merge (frontend work needs E2E; also eyeball the
  rendered knockout tip page — green e2e ≠ looks right).

## Confirmed wording

When `isKnockout` is true:

| Element | Group-stage (unchanged) | Knockout (new) |
|---|---|---|
| Reorder hint | "Drag tied teams to set your tiebreak order." | **"Predict the score after 90 minutes. If it's a draw, drag to pick who advances on extra time / penalties."** |
| Predicted heading | "Predicted standings" | **"Your pick"** |
| Actual heading | "Actual standings" | **"Result"** |

- The "after 90 minutes" + extra-time/penalties nuance is carried by the **hint**,
  not stuffed into the headings (confirmed). The hint renders only while editable
  (`!readOnly`), which is the correct place for the instruction.

### Columns (confirmed: simplify for knockout)

Drop the league-table columns `P / GD / Pts` for knockout rows. Show **rank with a
✓ on the team that advances + the team + its predicted goals**:

```
Your pick                    Result
#  Team       Goals          #  Team       Goals
✓  Brazil       3            ✓  Brazil       3
   Croatia      1               Croatia      1
```

- Per-team goals come from the existing `TeamStats` (`goalsFor` for that single
  match = the predicted/actual score). No new data needed.
- The ✓ marks rank 1 (the team that advances). Rank 2 is blank.
- Group-stage rendering of the same tables is **untouched** — implement as a
  `variant`/`isKnockout` branch in `StandingsTables.tsx`.

## Implementation outline

1. **`web/src/i18n/strings.ts`** — add three keys × **en + hu**:
   - `koAdvanceHint`, `koPredictedTitle`, `koActualTitle` (names illustrative).
   - Suggested Hungarian (Peter is a native speaker — confirm/adjust):
     - `koAdvanceHint`: "Tippeld meg a 90 perc utáni eredményt. Döntetlen esetén
       húzd a sorrendet, hogy eldöntsd, ki jut tovább hosszabbítás / büntetők után."
     - `koPredictedTitle`: "A tipped"
     - `koActualTitle`: "Eredmény"
2. **`web/src/pages/mytips/GroupTipForm.tsx`** — compute
   `const isKnockout = group.round !== 'GroupStage'`; pass it to
   `PredictedStandingsEditor`; pass a knockout-aware title to the actual
   `StandingsTable` (today it's hardcoded `t('actualStandings')` at ~line 348).
3. **`web/src/pages/mytips/StandingsTables.tsx`** — accept `isKnockout`; pick the
   heading (`koPredictedTitle` / `koActualTitle` vs `predictedStandings` /
   `actualStandings`), the hint (`koAdvanceHint` vs `drawOrderHint`), and render
   the simplified columns (rank ✓ + team + goals) when knockout.

## Acceptance criteria

- Group-stage tip pages are visually and textually identical to before.
- A knockout match tip page shows the three new strings (en + hu) and the
  simplified columns; no "standings", "tiebreak", or "Pts" wording remains in the
  knockout context.
- Reordering still submits the same `drawOrder` (behaviour unchanged); for a
  predicted draw the ✓ follows the top row.
- `npm run build`, `npm run lint`, and the e2e suite pass; the knockout tip page
  is visually verified.

## Reference (current code)

- Shared string: `web/src/i18n/strings.ts:182` (en), `:528` (hu `drawOrderHint`).
- Headings/hint render: `web/src/pages/mytips/StandingsTables.tsx:80-81` (predicted),
  parent passes actual title at `web/src/pages/mytips/GroupTipForm.tsx:347-351`.
- Knockout one-match groups in fixture: `tournaments/fwc26.json` — `KO-M*` leaves,
  `carries_standings: true`, `round` = `R32`/`R16`/`QF`/`SF`/`ThirdPlace`/`Final`.
