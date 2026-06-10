# My Tips — finalize countdown

**Date:** 2026-06-09
**Status:** approved, building

## Problem

My Tips lets a player enter and finalize predictions group by group, but gives
no sense of *how long is left* before a group locks. The deadline (a group's
earliest kickoff) is server-known but never surfaced. Players need a visible,
urgent reminder of when each group must be finalized.

## Decision

Show a **live-ticking countdown** to each group's finalize deadline, in two
places:

- **Page-level banner** under the My Tips heading: the soonest upcoming deadline
  among the player's visible, not-yet-finalized, not-past groups —
  "⏰ Next to finalize: {group} · {countdown}". Hidden when nothing is left to
  finalize, and for the result user.
- **Per-group**: in each `GroupTipForm` header, next to the existing
  draft/finalized badge, while the group is still open.

## Server-authoritative clock — compliance

The SPA's rule (CLAUDE.md): render server-derived time flags, never branch on
`Date.now()`. This countdown honors that:

- **Authority is unchanged.** Lock eligibility, `readOnly`, and `deadlinePassed`
  still come from the API exactly as today. The countdown adds no new locking
  logic.
- **The countdown is cosmetic.** The only `Date.now()` in the app lives inside
  one hook, `useServerClock`, which anchors to the GraphQL `now`
  (already returned by `TOURNAMENT_QUERY`, currently unused) and uses the client
  clock purely as a tick delta:
  `estimatedServerNow = serverNow + (Date.now() − clientTimeAtFetch)`.
  No logic branches on raw client time; the displayed number is anchored to the
  server's clock, so a wrong client clock cannot drift the value.
- **At zero, defer to the server.** When a countdown crosses zero the page
  refetches (`tournament` + `me`), so the server's `deadlinePassed` becomes the
  source of truth for locking. The client never unilaterally locks a group.

## Units

Small, focused, independently testable:

1. **`web/src/lib/countdown.ts`** — pure, no clock.
   - `clockSkewMs(serverNowIso, clientNowMs) → number` — `serverNow − clientNow`.
   - `formatCountdown(msRemaining) → string` — `"3d 04:11:22"` / `"01:59:48"`;
     returns a stable expired sentinel handled by the component when `≤ 0`.
   - `remainingMs(deadlineIso, estimatedServerNowMs) → number`.
2. **`web/src/lib/useServerClock.ts`** — `useServerClock(serverNowIso): number`.
   Captures skew once per `serverNowIso`, ticks every 1000ms via `setInterval`,
   returns the current estimated server-now ms. The sole `Date.now()` site.
3. **`web/src/components/Countdown.tsx`** — props `{ deadline, serverNowMs,
   onExpire? }`. Renders the ticking string via `formatCountdown`; when expired,
   renders the i18n `finalizeClosed` label and fires `onExpire` once. i18n'd.

## Wiring

- **`MyTipsPage`** — derive `serverNowMs = useServerClock(tournamentResult.data.now)`.
  Add the banner under `<h2>`: from the player's visible, not-finalized,
  not-past groups, choose the one with the soonest `deadline`; render group name
  + `<Countdown>`. Hidden when none / for the result user. `onExpire` →
  `refetchTournament({ requestPolicy: 'network-only' })` + `refetchMe(...)`.
  Pass `serverNowMs` to each `GroupTipForm`.
- **`GroupTipForm`** — in the `<h3>` header, next to the badge, render
  `<Countdown deadline={group.deadline} serverNowMs={...} />` while not locked
  and not past; the badge already covers locked/past states.

A group is "finalized" for banner purposes when every child game's
`MatchPrediction.locked` is true (the same signature MyTips already computes for
the form remount key), or `group.deadlinePassed`.

## i18n (EN + HU)

`nextToFinalize` ("Next to finalize") · `finalizeIn` ("finalize in") ·
`finalizeClosed` ("Finalize closed") · short unit label `daysShort` ("d") for
the `"Nd …"` form. Hungarian equivalents in `strings.ts`.

## Testing

- **Unit** `countdown.test.ts`: `formatCountdown` across ms inputs (sub-minute,
  hours, multi-day, exactly 0, negative → expired); `clockSkewMs`;
  `remainingMs`.
- **E2E** `mytips-countdown.spec.ts` (dev clock pins server `now`):
  - server now just before Group A's deadline → banner names "Group A" and shows
    a `HH:MM:SS` countdown.
  - server now past Group A's deadline → that group reads finalized/closed, no
    open countdown.

## Out of scope (YAGNI)

No notifications, no per-match countdowns, no timezone/stadium-local toggle (the
separate `timezone-clarity` idea), no sound/visual alarms.
