import type { SingleGame } from '../graphql/types'

/** A calendar-day section of the schedule, in the viewer's local timezone. */
export interface DaySection {
  /** Stable, sortable key for the local calendar day (sortable ISO `y-m-d`). */
  key: string
  /** Human-readable day heading, locale-formatted (e.g. "Sat, Jun 20, 2026"). */
  label: string
  /** Games kicking off on this local day, ordered by kickoff ascending. */
  games: SingleGame[]
}

/**
 * Calendar-day key for an ISO kickoff, in the viewer's LOCAL timezone — the
 * same basis `formatKickoff` renders against. Derived from `Intl` date parts
 * (not a string slice), so the day boundary is the viewer's local midnight.
 * Returns a sortable `YYYY-MM-DD` string; falls back to the raw input for an
 * unparseable date. Depends only on `iso` + `locale` — never on `Date.now()`.
 */
export function dayKey(iso: string, locale: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  const parts = new Intl.DateTimeFormat(locale, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }).formatToParts(d)
  const year = parts.find((p) => p.type === 'year')?.value ?? ''
  const month = parts.find((p) => p.type === 'month')?.value ?? ''
  const day = parts.find((p) => p.type === 'day')?.value ?? ''
  return `${year}-${month}-${day}`
}

/** Human-readable heading for a local calendar day. */
function dayLabel(iso: string, locale: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  return d.toLocaleDateString(locale, {
    weekday: 'short',
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
}

/**
 * Bucket a flat game list into ordered local-calendar-day sections. Sections
 * are ordered chronologically; games within each section are ordered by
 * kickoff ascending. Immutable — the input array is not mutated.
 */
export function groupByDay(games: SingleGame[], locale: string): DaySection[] {
  const sorted = [...games].sort(
    (a, b) => Date.parse(a.kickoff) - Date.parse(b.kickoff),
  )
  const byKey = new Map<string, DaySection>()
  for (const g of sorted) {
    const key = dayKey(g.kickoff, locale)
    const existing = byKey.get(key)
    if (existing) {
      byKey.set(key, { ...existing, games: [...existing.games, g] })
    } else {
      byKey.set(key, { key, label: dayLabel(g.kickoff, locale), games: [g] })
    }
  }
  return [...byKey.values()].sort((a, b) => a.key.localeCompare(b.key))
}
