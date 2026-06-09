import { describe, expect, it } from 'vitest'
import { stashReturnTo, takeReturnTo } from './returnTo'

/** A Map-backed Storage stub — vitest's node env has no real sessionStorage. */
function fakeStorage(): Storage {
  const m = new Map<string, string>()
  return {
    get length() {
      return m.size
    },
    clear: () => m.clear(),
    getItem: (k: string) => (m.has(k) ? (m.get(k) as string) : null),
    key: (i: number) => Array.from(m.keys())[i] ?? null,
    removeItem: (k: string) => {
      m.delete(k)
    },
    setItem: (k: string, v: string) => {
      m.set(k, v)
    },
  }
}

describe('returnTo', () => {
  it('round-trips a stashed path', () => {
    const s = fakeStorage()
    stashReturnTo('/invite/ABC123', s)
    expect(takeReturnTo(s)).toBe('/invite/ABC123')
  })

  it('is one-shot — clears after taking', () => {
    const s = fakeStorage()
    stashReturnTo('/invite/ABC123', s)
    takeReturnTo(s)
    expect(takeReturnTo(s)).toBeNull()
  })

  it('returns null when nothing is stashed', () => {
    expect(takeReturnTo(fakeStorage())).toBeNull()
  })
})
