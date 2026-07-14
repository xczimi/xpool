import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fetchWithAuth } from './client'
import { clearSessionExpired, isSessionExpired } from '../auth/sessionState'
import { setAuth0Getter } from '../auth/devAuth'

const realFetch = globalThis.fetch

describe('fetchWithAuth', () => {
  beforeEach(() => {
    clearSessionExpired()
    setAuth0Getter(() => Promise.resolve('a-token'))
  })

  afterEach(() => {
    globalThis.fetch = realFetch
    setAuth0Getter(null)
  })

  it('attaches the bearer token and leaves a healthy session alone', async () => {
    const spy = vi.fn<typeof fetch>(async () => new Response('{}', { status: 200 }))
    globalThis.fetch = spy as unknown as typeof fetch

    await fetchWithAuth('/api/graphql', { method: 'POST' })

    const headers = (spy.mock.calls[0][1] as RequestInit).headers as Headers
    expect(headers.get('Authorization')).toBe('Bearer a-token')
    expect(isSessionExpired()).toBe(false)
  })

  it('marks the session expired when the seam rejects the token with 401', async () => {
    globalThis.fetch = (async () =>
      new Response('invalid token', { status: 401 })) as unknown as typeof fetch

    const res = await fetchWithAuth('/api/graphql', { method: 'POST' })

    expect(res.status).toBe(401)
    expect(isSessionExpired()).toBe(true)
  })
})
