import { beforeEach, describe, expect, it } from 'vitest'
import { resolveToken, setAuth0Getter } from './devAuth'
import { clearSessionExpired, isSessionExpired } from './sessionState'

describe('resolveToken', () => {
  beforeEach(() => {
    clearSessionExpired()
    setAuth0Getter(null)
  })

  it('returns the Auth0 token when the silent refresh works', async () => {
    setAuth0Getter(() => Promise.resolve('fresh-token'))
    expect(await resolveToken()).toBe('fresh-token')
    expect(isSessionExpired()).toBe(false)
  })

  // The production failure: the refresh token is gone, so the SDK rejects.
  it('marks the session expired when the silent refresh rejects', async () => {
    setAuth0Getter(() => Promise.reject(new Error('login_required')))
    expect(await resolveToken()).toBeNull()
    expect(isSessionExpired()).toBe(true)
  })
})
