# Dev clock — game-relative presets — design

**Date:** 2026-06-06
**Status:** approved

## Goal

Replace the dev clock's free-form `datetime-local` input with two dropdowns that
set the server clock (`X-Dev-Now`) to a time relative to a chosen game. This
makes it one-step to land on the three time-dependent states the UI cares about
(predictions open, match in progress, match over) without hand-typing a
timestamp.

The dev clock is dev-only UI (the `DevAuthBar` path; absent when Auth0 is
enabled). The server-authoritative clock model is unchanged — this only changes
how a dev picks the instant; it still flows through `setDevNow(iso)` →
`X-Dev-Now` header → server `now`.

## UX

Two native `<select>`s plus the existing Reset button:

- **Game** — every game, ordered by kickoff ascending. Label:
  `Jun 11 18:00 · M1 · MEX v ???`. Kickoff is shown in the **browser's local
  time** for readability; the value applied is the exact UTC instant. The team is
  the resolved `shortCode`, falling back to the slot `description`
  ("Winner Group A") when the team is undetermined.
- **When** — three fixed entries, disabled until a game is chosen:
  - `10 min before kickoff`
  - `during (60 min in)`
  - `15 min after full time`

Changing either select, once **both** are chosen, computes the instant and calls
`setDevNow(iso)` then reloads (same mechanism as today). A text line shows the
current effective dev time with **Reset** beside it. The selects are controls,
not state mirrors: on reload they return to placeholders and the active time is
shown as text only.

## Time computation

A pure, unit-tested helper — `web/src/components/devClockTimes.ts`:

```ts
export type DevClockPhase = 'before' | 'during' | 'after'

/** Instant (RFC3339 UTC) for a phase relative to a game's kickoff K. */
export function devClockInstant(kickoffIso: string, phase: DevClockPhase): string
```

Offsets from kickoff `K`:

| Phase    | Instant   | Intent                                   |
|----------|-----------|------------------------------------------|
| before   | K − 10 m  | predictions still open (deadline future) |
| during   | K + 60 m  | kicked off, result pending               |
| after    | K + 135 m | ~2h match over (90' + halftime + stoppage), +15 m |

**Assumption:** the domain models only `kickoff` (no game end/duration), so match
length is fixed at ~2h. For a knockout decided in extra-time + penalties,
"+135m after" is nominal, not a guaranteed post-final-whistle instant. Acceptable
for a dev preset.

## Components & data

- Extract the dev clock out of `web/src/components/AuthBar.tsx` into its own
  `web/src/components/DevClock.tsx`. `AuthBar` renders `<DevClock />` where it
  currently inlines the `DevClock` function.
- `DevClock` runs a slim dedicated query (not the heavy `TOURNAMENT_QUERY`):

  ```graphql
  query DevClockGames {
    tournament {
      games { id kickoff home { teamId description } away { teamId description } }
      teams { id shortCode }
    }
  }
  ```

  urql caches it; it mounts only in dev mode, so there is no production cost.

## i18n

New strings in `web/src/i18n/strings.ts` (en + hu): the two select labels, both
placeholders, and the three phase names. The existing `devClock` /
`devClockReset` strings are reused.

## Testing

- **Unit** — `devClockInstant`: each phase's offset and UTC correctness
  (e.g. a kickoff near a DST boundary still yields the right UTC instant).
- **E2E** (Playwright, one spec) — log in as a seeded player, pick a game and the
  `during` phase, assert the clock took effect: that game's group reports
  `deadlinePassed` (predictions locked) / the SPA reflects a kicked-off state.
  Per the project rule that frontend work gets end-to-end coverage.

## Out of scope

- No change to the server clock resolution or `X-Dev-Now` contract.
- No reverse-mapping of an arbitrary current dev time back onto a (game, phase)
  selection — the dropdowns are write-only controls.
- No persistence of the last selection beyond the existing `xpool.devNow`.
