import { describe, expect, it } from 'vitest'
import {
  clockSkewMs,
  formatAbsoluteDeadline,
  formatRelative,
  remainingMs,
} from './countdown'

const secs = (n: number) => n * 1000

describe('formatRelative', () => {
  describe('under an hour — ticking MM:SS (the urgency zone)', () => {
    it('formats sub-minute as MM:SS', () => {
      expect(formatRelative(secs(45), 'en')).toBe('00:45')
    })

    it('formats minutes + seconds as MM:SS', () => {
      expect(formatRelative(secs(32 * 60 + 7), 'en')).toBe('32:07')
    })

    it('just under an hour is still MM:SS', () => {
      expect(formatRelative(secs(59 * 60 + 59), 'en')).toBe('59:59')
    })

    it('clamps zero / negative to 00:00', () => {
      expect(formatRelative(0, 'en')).toBe('00:00')
      expect(formatRelative(-5000, 'en')).toBe('00:00')
    })
  })

  describe('one hour to a day — hours + minutes, no seconds', () => {
    it('formats exactly one hour with no minute part', () => {
      expect(formatRelative(secs(3600), 'en')).toBe('in 1h')
    })

    it('formats hours and minutes compactly', () => {
      expect(formatRelative(secs(5 * 3600 + 32 * 60), 'en')).toBe('in 5h 32m')
    })

    it('drops the seconds within the hours tier', () => {
      // 5h 32m 49s → still 5h 32m
      expect(formatRelative(secs(5 * 3600 + 32 * 60 + 49), 'en')).toBe(
        'in 5h 32m',
      )
    })

    it('just under a day stays in the hours tier', () => {
      expect(formatRelative(secs(23 * 3600 + 59 * 60), 'en')).toBe('in 23h 59m')
    })

    it('localises hours + minutes (hu)', () => {
      expect(formatRelative(secs(5 * 3600 + 32 * 60), 'hu')).toBe(
        '5 ó 32 p múlva',
      )
    })
  })

  describe('a day or more — whole days, no ticking', () => {
    it('formats exactly one day', () => {
      expect(formatRelative(secs(86_400), 'en')).toBe('in 1 day')
    })

    it("the original '1d 00:32:37' complaint collapses to 'in 1 day'", () => {
      expect(formatRelative(secs(86_400 + 32 * 60 + 37), 'en')).toBe('in 1 day')
    })

    it('floors to whole days and pluralises', () => {
      expect(formatRelative(secs(3 * 86_400 + 4 * 3600), 'en')).toBe(
        'in 3 days',
      )
    })

    it('localises days (hu)', () => {
      expect(formatRelative(secs(3 * 86_400), 'hu')).toBe('3 nap múlva')
    })
  })
})

describe('formatAbsoluteDeadline', () => {
  // Pin the zone so the local-day boundary is deterministic across machines.
  const utc = 'UTC'

  it("renders 'today HH:MM' when the deadline is the local same day", () => {
    const deadline = '2026-06-11T18:00:00Z'
    const now = Date.parse('2026-06-11T08:00:00Z')
    expect(formatAbsoluteDeadline(deadline, now, 'en', utc)).toBe('today 18:00')
  })

  it('renders a dated label when the deadline is a different local day', () => {
    const deadline = '2026-06-13T18:00:00Z' // a Saturday
    const now = Date.parse('2026-06-11T08:00:00Z')
    expect(formatAbsoluteDeadline(deadline, now, 'en', utc)).toBe(
      'Sat, Jun 13, 18:00',
    )
  })

  it("localises 'today' (hu)", () => {
    const deadline = '2026-06-11T18:00:00Z'
    const now = Date.parse('2026-06-11T08:00:00Z')
    expect(formatAbsoluteDeadline(deadline, now, 'hu', utc)).toBe('ma 18:00')
  })

  it('decides today in the supplied zone, not UTC', () => {
    // 01:00Z June 12 is still June 11 in New York (EDT, UTC-4); now is that
    // same evening there — so the deadline reads as "today" in-zone.
    const deadline = '2026-06-12T01:00:00Z' // 21:00 on Jun 11 in America/New_York
    const now = Date.parse('2026-06-11T22:00:00Z') // 18:00 Jun 11 in NY
    expect(
      formatAbsoluteDeadline(deadline, now, 'en', 'America/New_York'),
    ).toBe('today 21:00')
  })
})

describe('clockSkewMs', () => {
  it('is the signed offset of server now from the client clock', () => {
    const serverNow = '2026-06-11T12:00:00Z'
    const clientNow = Date.parse('2026-06-11T11:59:58Z') // client 2s behind
    expect(clockSkewMs(serverNow, clientNow)).toBe(2_000)
  })
})

describe('remainingMs', () => {
  it('is the deadline minus the estimated server now', () => {
    const deadline = '2026-06-11T12:00:00Z'
    const estNow = Date.parse('2026-06-11T11:00:00Z')
    expect(remainingMs(deadline, estNow)).toBe(3_600_000)
  })

  it('goes negative once the deadline has passed', () => {
    const deadline = '2026-06-11T12:00:00Z'
    const estNow = Date.parse('2026-06-11T12:00:10Z')
    expect(remainingMs(deadline, estNow)).toBe(-10_000)
  })
})
