# Humane finalize countdown — design

**Date:** 2026-06-10
**Status:** approved, implementing on branch `humane-countdown`

## Problem

The My Tips finalize countdown renders a second-by-second duration regardless of
how far away the deadline is, e.g. `Group A Draft · finalize in 1d 00:32:37`. A
ticking seconds field is noise when the deadline is over a day out, and the label
gives no absolute reference — the user can't tell *when* 18:00 actually is in
their own local time without doing the arithmetic.

## Goals

1. Scale the relative granularity to urgency — no ticking seconds until the final
   hour.
2. Always show the absolute deadline in the user's **local** time.
3. Keep the clock server-authoritative (no new `Date.now()` gates; locking stays
   driven by `deadlinePassed`). The countdown remains display-only.

## Format scheme — "absolute + scaled relative"

Label: `{group} · finalize by {absolute} — {relative}`

### Relative granularity tiers (remaining time `r`)

| `r`        | Relative      | Display refresh        |
|------------|---------------|------------------------|
| ≥ 24h      | `in 3 days`   | recompute / minute (string changes daily) |
| 1h – 24h   | `in 5h 32m`   | per minute             |
| 0 < r < 1h | `32:07` (MM:SS) | per second (urgency zone) |
| r ≤ 0      | `finalizeClosed` label | — |

- Days are floored and pluralized (`in 1 day`, `in 3 days`).
- Under an hour the redundant `00:` hour is dropped → `MM:SS`.
- The existing `useServerClock` 1s tick is kept (cosmetic, shared); far out the
  formatted string simply doesn't change per-second, which satisfies the ask
  with no new timer machinery. Throttling the interval by tier is a possible
  later optimisation, not required.

### Absolute deadline (always shown, local time, locale-aware)

- Deadline falls on the user's local **today** → `today 18:00`.
- Otherwise → `Wed 11 Jun, 18:00` (short weekday + day + short month + 24h time).
- Rendered via `toLocaleDateString` / `toLocaleTimeString` with the active i18n
  locale (`en` / `hu`). Formatting a fixed server-provided instant in the
  browser's timezone is legitimate — it does not read `Date.now()` and is not a
  locking decision.

### Examples

```
Group A Draft · finalize by Wed 11 Jun, 18:00 — in 1 day
Group A Draft · finalize by Wed 11 Jun, 18:00 — in 5h 32m
Group A Draft · finalize by today 18:00 — 32:07
```

## Implementation surface

- **`web/src/lib/countdown.ts`** — replace the single `formatCountdown` with two
  pure helpers (clock-free; locale + relative-time strings passed in by caller):
  - `formatRelative(msRemaining, fmt)` → tiered relative string.
  - `formatAbsoluteDeadline(deadlineIso, serverNowMs, locale)` → `today 18:00` /
    `Wed 11 Jun, 18:00`, deciding "today" against the server-anchored now.
  `remainingMs` / `clockSkewMs` stay as-is.
- **`web/src/lib/countdown.test.ts`** — unit tests drive every tier boundary
  (just-under-1h, exactly-1h, just-under-24h, multi-day) and the today/dated
  absolute split, plus a Hungarian-locale case.
- **`web/src/components/Countdown.tsx`** — render `by {absolute} — {relative}`;
  keep `onExpire` and the expired→`finalizeClosed` path. Pulls locale + relative
  wording from `useI18n`.
- **`web/src/i18n/strings.ts`** — add `finalizeBy`, `today`, and relative day /
  hour-minute wording for en + hu. Day/hour wording leans on
  `Intl.RelativeTimeFormat(locale)` where it reads naturally (`1 nap múlva`).
- **E2E** — drive `X-Dev-Now` to place the active group's deadline far out
  (assert no ticking seconds, absolute shown) and under an hour (assert MM:SS).

## Out of scope

- Per-tier interval throttling (cosmetic perf only).
- A `tomorrow` special-case (only `today` is special-cased; everything else is
  dated).
- Changing the locking / `deadlinePassed` server authority.
