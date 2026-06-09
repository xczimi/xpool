# Hide future rounds that aren't ready for predictions

**Date:** 2026-06-09
**Status:** Approved design — ready for implementation plan

## Problem

Knockout rounds (R32 → Final) appear in the UI from the start of the
tournament, showing placeholder slots like `"Winner SF 1"` / `"3ABCDF"`. A
player sees — and can interact with — rounds whose participants are not yet
known. We want to stop surfacing a future round until it is actually ready for
predictions.

**Definition of "ready":** a round is ready once there is at least one game in
it whose *both* teams are determined. Until then, the round is hidden.

## Decisions

These were settled during brainstorming and are load-bearing:

1. **Readiness basis — official results only.** A slot becomes determined only
   when the official results (the result-user's predictions) resolve it. A
   player's own predictions never resolve who is playing. This is already how
   the system works: `recompute.rs` runs `fwc26::resolve_bracket` against the
   official results and **materialises** resolved `team_id`s onto the stored
   knockout games. The SPA therefore already receives real `teamId`s on games
   the moment they are known — readiness needs no new server computation.

2. **Scope — My Tips and Scoreboard only.** The Schedule/fixtures page keeps
   showing every game (it is the full fixture list, placeholders included).

3. **Granularity — round-level only.** Once a round qualifies (≥1 game with both
   teams known), the **whole** round shows, including any games inside it still
   carrying placeholder slots. We do not hide individual unresolved games within
   an open round.

4. **No server-side guard.** The submit path (`submitGroup`) has **no** check
   that a game's teams are resolved, and we are deliberately **not** adding one.
   Consistent with the project's soft-funnel philosophy (see the invite-auth
   funnel), this change is a UI funnel, not a hard gate. See "Accepted gap".

## The rule, in one place

A single pure helper in `web/src/lib/rounds.ts`:

```ts
export function readyRounds(groups: GroupGame[], games: Game[]): Set<Round>
```

In one pass it walks every game whose `home.teamId` **and** `away.teamId` are
truthy, maps each through `game.groupId → group.round`, and collects the set of
rounds reached. A round is ready iff it is in the set.

- Group Stage games carry real teams from import, so `GROUP_STAGE` is always in
  the set — no special-casing needed; the generic rule covers it.
- The set only ever grows as the tournament progresses, so a round that has
  become visible never disappears underneath the user.

No GraphQL schema change. No backend change. The data is already on the wire.

## Surface 1 — My Tips (`RoundNav` + `MyTipsPage`)

- `RoundNav` gains a `games: Game[]` prop. It filters `roundNodes(groups)` down
  to nodes whose `round ∈ readyRounds(groups, games)`. A hidden round is a
  hidden tab.
- `MyTipsPage` runs its default/active-round selection (`currentRoundNode`) over
  the **filtered** list, so the page never auto-lands on a hidden round. Because
  the ready-set only grows, the currently-selected round never vanishes.

## Surface 2 — Scoreboard (`ScoreboardPage`)

The scoreboard renders one **column per round** from a hard-coded `ROUND_ORDER`
(both the header row and each player row's cells).

- Compute `ready = readyRounds(groups, games)` and replace the two
  `ROUND_ORDER.map(...)` sites with `ROUND_ORDER.filter(r => ready.has(r))` (map
  over the filtered list). A hidden round is a dropped column.
- `GROUP_STAGE` always survives; `THIRD_PLACE` and `FINAL` are subject to the
  same rule.
- `ScoreboardPage` reads tournament `groups` + `games` via the existing
  `TOURNAMENT_QUERY` (urql-cached — already fetched by other pages) to compute
  `ready`.

## Accepted gap

Because display is round-level only and there is no server guard:

> When a round is open but only some of its games have determined teams, the
> still-placeholder games inside that round are visible and remain technically
> saveable/lockable via `submitGroup`.

This is an accepted, documented trade-off — a soft UI funnel, not a hard gate.
If it ever needs closing, the follow-up is a server-side guard in `submitGroup`
that rejects a `MatchPrediction` whose game has an unresolved slot, optionally
paired with hiding unresolved games within an open round. Out of scope here.

## Testing

- **Unit (`web/src/lib/rounds.ts`):**
  - group-only fixture → `readyRounds` is `{ GROUP_STAGE }`.
  - fixture with one R32 game's both slots resolved → set includes `R32`.
  - knockout games with placeholder-only slots → those rounds excluded.
- **Component (`RoundNav`):** given placeholder-only knockout games, only the
  Group Stage tab renders.
- **E2E:** on the default seed (knockouts unresolved), My Tips shows no
  R32–Final tabs and the Scoreboard shows no knockout columns. (Frontend work is
  always verified end-to-end.)

## Out of scope

- Any server-side validation of predictions against team resolution.
- Hiding individual unresolved games within an open round.
- Changes to the Schedule/fixtures page.
