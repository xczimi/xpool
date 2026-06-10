import { describe, expect, it } from 'vitest'
import {
  rememberPendingInvite,
  takePendingInvitePath,
} from './pendingInvite'

/** A Map-backed Storage stub — vitest's node env has no real localStorage. */
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

describe('pendingInvite', () => {
  it('round-trips a remembered code as a claim path', () => {
    const s = fakeStorage()
    rememberPendingInvite('ABC123', s)
    expect(takePendingInvitePath(s)).toBe('/invite/ABC123')
  })

  it('is one-shot — clears after taking', () => {
    const s = fakeStorage()
    rememberPendingInvite('ABC123', s)
    takePendingInvitePath(s)
    expect(takePendingInvitePath(s)).toBeNull()
  })

  it('returns null when nothing is remembered', () => {
    expect(takePendingInvitePath(fakeStorage())).toBeNull()
  })
})
