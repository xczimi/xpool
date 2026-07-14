import { describe, expect, it, beforeEach } from 'vitest'
import {
  clearSessionExpired,
  isSessionExpired,
  markSessionExpired,
  subscribeSessionExpired,
} from './sessionState'

describe('sessionState', () => {
  beforeEach(() => {
    clearSessionExpired()
  })

  it('starts un-expired', () => {
    expect(isSessionExpired()).toBe(false)
  })

  it('marks and clears', () => {
    markSessionExpired()
    expect(isSessionExpired()).toBe(true)
    clearSessionExpired()
    expect(isSessionExpired()).toBe(false)
  })

  it('notifies subscribers on change', () => {
    let calls = 0
    const unsubscribe = subscribeSessionExpired(() => {
      calls += 1
    })
    markSessionExpired()
    expect(calls).toBe(1)
    unsubscribe()
    clearSessionExpired()
    expect(calls).toBe(1)
  })

  it('does not notify when the value is unchanged (guards a render loop)', () => {
    let calls = 0
    const unsubscribe = subscribeSessionExpired(() => {
      calls += 1
    })
    markSessionExpired()
    markSessionExpired()
    expect(calls).toBe(1)
    unsubscribe()
  })
})
