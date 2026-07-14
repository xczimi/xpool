import { describe, expect, it } from 'vitest'
import { contentGate } from './contentGate'

describe('contentGate', () => {
  it('renders the page for a healthy player', () => {
    expect(
      contentGate({ access: 'player', sessionExpired: false, hasSession: true, viewer: 'player' }),
    ).toBe('page')
  })

  it('shows session-expired when a detector has fired', () => {
    expect(
      contentGate({ access: 'player', sessionExpired: true, hasSession: true, viewer: 'anonymous' }),
    ).toBe('session-expired')
  })

  // A visitor who never logged in but carries a stray token in localStorage:
  // the 401 marks the flag globally, but they never had a session, so telling
  // them it "expired" is a false positive. They get the page's login prompt.
  it('does not tell a never-logged-in visitor their session expired', () => {
    expect(
      contentGate({ access: 'player', sessionExpired: true, hasSession: false, viewer: 'anonymous' }),
    ).toBe('page')
  })

  // Detector 3: the client thinks it is logged in, the server says Visitor.
  // This is the exact production state behind the bare "Something went wrong."
  it('shows session-expired when the client has a session but `me` is null', () => {
    expect(
      contentGate({ access: 'player', sessionExpired: false, hasSession: true, viewer: 'anonymous' }),
    ).toBe('session-expired')
  })

  it('gates admin routes on a dead session too', () => {
    expect(
      contentGate({ access: 'admin', sessionExpired: true, hasSession: true, viewer: 'anonymous' }),
    ).toBe('session-expired')
  })

  it('leaves public pages reachable with a dead session', () => {
    expect(
      contentGate({ access: 'public', sessionExpired: true, hasSession: false, viewer: 'anonymous' }),
    ).toBe('page')
  })

  // Regression guard: the invite dead-end must survive this change.
  it('still shows needs-invite for an unclaimed viewer with no link candidate', () => {
    expect(
      contentGate({ access: 'player', sessionExpired: false, hasSession: true, viewer: 'unclaimed' }),
    ).toBe('needs-invite')
  })

  it('does not dead-end an unclaimed viewer who has a link candidate', () => {
    expect(
      contentGate({
        access: 'player',
        sessionExpired: false,
        hasSession: true,
        viewer: 'unclaimed-linkable',
      }),
    ).toBe('page')
  })

  it('does not flash session-expired while `me` is still in flight', () => {
    expect(
      contentGate({ access: 'player', sessionExpired: false, hasSession: true, viewer: 'loading' }),
    ).toBe('page')
  })

  // A logged-out visitor on a player route is not "expired" — the page itself
  // renders NeedsLogin. Preserves today's behaviour.
  it('renders the page for a visitor with no session', () => {
    expect(
      contentGate({ access: 'player', sessionExpired: false, hasSession: false, viewer: 'loading' }),
    ).toBe('page')
  })
})
