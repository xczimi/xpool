/**
 * Pure countdown arithmetic for the My Tips finalize deadline — no clock, no
 * I/O, so it is fully unit-testable. The live clock anchoring lives in
 * `useServerClock`; this module only does math and formatting.
 *
 * Server-authoritative clock (CLAUDE.md): nothing here reads `Date.now()`. The
 * caller supplies an already-estimated server-now (anchored to the GraphQL
 * `now`), so the formatted countdown tracks the server's clock, not the
 * browser's. Formatting a fixed, server-provided deadline instant in the
 * viewer's timezone is display-only and never gates locking.
 */

import type { Locale } from '../i18n/strings'

const pad2 = (n: number): string => String(n).padStart(2, '0')

/** Signed offset of the server clock from the client clock, in ms. */
export function clockSkewMs(serverNowIso: string, clientNowMs: number): number {
  return Date.parse(serverNowIso) - clientNowMs
}

/** Milliseconds until `deadlineIso`, given an estimated server-now in ms. */
export function remainingMs(
  deadlineIso: string,
  estimatedServerNowMs: number,
): number {
  return Date.parse(deadlineIso) - estimatedServerNowMs
}

/**
 * Format the remaining time with a granularity that scales to urgency, so a
 * deadline days away never shows a twitching seconds field:
 *
 * - `>= 1 day`   → `in 3 days` (whole days, floored, localised, no ticking)
 * - `1h .. 24h`  → `in 5h 32m` (hours + minutes, no seconds)
 * - `< 1h`       → `32:07` (MM:SS — the only tier that ticks per second)
 *
 * Negative input clamps to `00:00`; callers treat `<= 0` as expired and render
 * the closed label instead.
 */
export function formatRelative(msRemaining: number, locale: Locale): string {
  const totalSeconds = Math.max(0, Math.floor(msRemaining / 1000))

  if (totalSeconds >= 86_400) {
    const days = Math.floor(totalSeconds / 86_400)
    return new Intl.RelativeTimeFormat(locale, { numeric: 'always' }).format(
      days,
      'day',
    )
  }

  if (totalSeconds >= 3600) {
    const hours = Math.floor(totalSeconds / 3600)
    const minutes = Math.floor((totalSeconds % 3600) / 60)
    const unit = (n: number, u: 'hour' | 'minute') =>
      new Intl.NumberFormat(locale, {
        style: 'unit',
        unit: u,
        unitDisplay: 'narrow',
      }).format(n)
    const body =
      minutes > 0
        ? `${unit(hours, 'hour')} ${unit(minutes, 'minute')}`
        : unit(hours, 'hour')
    // Hungarian places the relative marker after the duration ("… múlva");
    // English before it ("in …"). Only these two locales exist.
    return locale === 'hu' ? `${body} múlva` : `in ${body}`
  }

  const minutes = Math.floor(totalSeconds / 60)
  const seconds = totalSeconds % 60
  return `${pad2(minutes)}:${pad2(seconds)}`
}

const localDayKey = (ms: number, timeZone?: string): string =>
  // `en-CA` yields an ISO-ish `YYYY-MM-DD`, so string equality == same local day.
  new Date(ms).toLocaleDateString('en-CA', timeZone ? { timeZone } : undefined)

/**
 * The absolute deadline in the viewer's local time, e.g. `Sat, Jun 13, 18:00`,
 * collapsing to `today 18:00` when it falls on the viewer's current local day.
 * `timeZone` is for deterministic tests; production passes none → browser zone.
 */
export function formatAbsoluteDeadline(
  deadlineIso: string,
  serverNowMs: number,
  locale: Locale,
  timeZone?: string,
): string {
  const deadlineMs = Date.parse(deadlineIso)
  const tz = timeZone ? { timeZone } : undefined

  const time = new Date(deadlineMs).toLocaleTimeString(locale, {
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
    ...tz,
  })

  if (localDayKey(deadlineMs, timeZone) === localDayKey(serverNowMs, timeZone)) {
    const today = new Intl.RelativeTimeFormat(locale, {
      numeric: 'auto',
    }).format(0, 'day') // "today" / "ma"
    return `${today} ${time}`
  }

  const date = new Date(deadlineMs).toLocaleDateString(locale, {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
    ...tz,
  })
  return `${date}, ${time}`
}
