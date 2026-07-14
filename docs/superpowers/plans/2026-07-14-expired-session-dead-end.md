# Expired-session dead-end — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A player whose Auth0 session has expired sees "Your session has expired — log in again" with a working button, instead of a bare "Something went wrong." on every player page.

**Architecture:** Three failures that currently degrade silently (Auth0 silent-refresh rejection, a `401` from the auth seam, and `me` resolving to `null` while the SPA still shows a login) all funnel into one module-level `sessionExpired` flag. `Layout` consults a pure `contentGate()` function and swaps a new `SessionExpired` view in for `<Outlet />` on non-public routes — exactly how the existing `NeedsInvite` dead-end works. When the flag is set, `label` becomes `null`, so every `pause: !label` query stops firing and the auth bar stops claiming the user is logged in.

**Tech stack:** React 19 + TypeScript + urql + Auth0 SPA SDK; vitest (node environment, `*.test.ts` only — there is **no** jsdom/testing-library, so all unit tests here are on pure functions); Playwright for e2e.

**Spec:** `docs/superpowers/specs/2026-07-14-expired-session-dead-end-design.md`

---

## Guardrails (read before starting)

- **Branch discipline (CLAUDE.md):** this touches `web/` source, so it may **not** be committed to `master` directly. Work on a branch or git worktree, merge to `master` locally when done.
- **Never rewrite git history.** No `amend`, no `rebase`, no `reset`. Forward commits only.
- The e2e suite boots its own isolated stack on dynamic ports (`npm run e2e`); it coexists with a running `bin/local-dev` session. Do not tear down the `:3000`/`:5173` stack.

## File structure

| File | Responsibility |
|---|---|
| `web/src/auth/sessionState.ts` *(new)* | The module-level flag + listener registry. Lives outside React because a detector fires inside `fetchWithAuth`. |
| `web/src/auth/sessionState.test.ts` *(new)* | Unit tests for the above. |
| `web/src/auth/contentGate.ts` *(new)* | Pure decision: given the route access + session + viewer, render the page, the session-expired view, or the invite dead-end. |
| `web/src/auth/contentGate.test.ts` *(new)* | Unit tests for the gate, incl. a regression guard on the invite dead-end. |
| `web/src/auth/devAuth.ts` | Detector 1: `resolveToken()` stops swallowing a silent-refresh failure. |
| `web/src/auth/devAuth.test.ts` *(new)* | Unit test for detector 1. |
| `web/src/graphql/client.ts` | Detector 2: a `401` response marks the session expired. Export `fetchWithAuth` so it is testable. |
| `web/src/graphql/client.test.ts` *(new)* | Unit test for detector 2 with a stubbed global `fetch`. |
| `web/src/auth/authContextValue.ts` | `AuthState` gains `sessionExpired` + `reauthenticate()`. |
| `web/src/auth/AuthContext.tsx` | Both providers: null the label when expired, mark expired on silent-refresh failure, implement `reauthenticate()`. |
| `web/src/components/SessionExpired.tsx` *(new)* | The view. Sibling of `NeedsInvite`. |
| `web/src/components/Layout.tsx` | Detector 3 + renders `SessionExpired` via `contentGate()`. |
| `web/src/components/AuthBar.tsx` | `ProdAuthBar` keys off Auth0's `isAuthenticated`, **not** `label` — it must respect `sessionExpired` too, or it keeps showing "Logged in as …". |
| `web/src/i18n/strings.ts` | 3 new keys × 2 locales. |
| `web/src/index.css` | `.session-expired` styling (mirrors `.needs-invite`). |
| `web/e2e/session-expired.spec.ts` *(new)* | End-to-end: junk JWT → 401 → the view, against the real API. |

---

### Task 1: The `sessionExpired` flag

**Files:**
- Create: `web/src/auth/sessionState.ts`
- Test: `web/src/auth/sessionState.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
// web/src/auth/sessionState.test.ts
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
```

- [ ] **Step 2: Run the test — it must fail**

Run: `cd web && npm test -- sessionState`
Expected: FAIL — `Failed to resolve import "./sessionState"`.

- [ ] **Step 3: Implement**

```ts
// web/src/auth/sessionState.ts
/**
 * The one "the server no longer accepts this session" flag.
 *
 * It lives outside React because one of its writers is the urql fetch wrapper
 * (`graphql/client.ts`), which runs outside the component tree. `AuthContext`
 * reads it through `useSyncExternalStore`.
 *
 * Set by: an Auth0 silent-refresh rejection (`devAuth.resolveToken`), a 401
 * from the auth seam (`graphql/client.ts`), and `me` resolving to null while
 * the SPA still believes it is logged in (`components/Layout.tsx`).
 */

type Listener = () => void

let expired = false
const listeners = new Set<Listener>()

function notify(): void {
  for (const listener of listeners) listener()
}

/** The session cannot authenticate any more. Idempotent. */
export function markSessionExpired(): void {
  if (expired) return
  expired = true
  notify()
}

/** A fresh, working session exists again. Idempotent. */
export function clearSessionExpired(): void {
  if (!expired) return
  expired = false
  notify()
}

export function isSessionExpired(): boolean {
  return expired
}

/** `useSyncExternalStore` subscribe: returns the unsubscribe function. */
export function subscribeSessionExpired(listener: Listener): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}
```

- [ ] **Step 4: Run the test — it must pass**

Run: `cd web && npm test -- sessionState`
Expected: PASS, 4 tests.

- [ ] **Step 5: Commit**

```bash
git add web/src/auth/sessionState.ts web/src/auth/sessionState.test.ts
git commit -m "feat(web): add the sessionExpired flag store"
```

---

### Task 2: The `contentGate` decision (detector 3's logic, as a pure function)

Layout must decide between three things. Keeping the decision pure is what makes it testable at all — the vitest setup is node-only, so a rendered-component test is not available.

**Files:**
- Create: `web/src/auth/contentGate.ts`
- Test: `web/src/auth/contentGate.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
// web/src/auth/contentGate.test.ts
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
      contentGate({ access: 'player', sessionExpired: true, hasSession: false, viewer: 'anonymous' }),
    ).toBe('session-expired')
  })

  // Detector 3: the client thinks it is logged in, the server says Visitor.
  // This is the exact production state behind the bare "Something went wrong."
  it('shows session-expired when the client has a session but `me` is null', () => {
    expect(
      contentGate({ access: 'player', sessionExpired: false, hasSession: true, viewer: 'anonymous' }),
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
```

- [ ] **Step 2: Run the test — it must fail**

Run: `cd web && npm test -- contentGate`
Expected: FAIL — `Failed to resolve import "./contentGate"`.

- [ ] **Step 3: Implement**

```ts
// web/src/auth/contentGate.ts
import type { Access } from './routeAccess'

/** What the `me` query says about the viewer, once resolved. */
export type ViewerState =
  /** The `me` query has not settled yet (or is paused). */
  | 'loading'
  /** A real Player. */
  | 'player'
  /** Authenticated, not a Player, no link candidate — the invite dead-end. */
  | 'unclaimed'
  /** Authenticated, not a Player, but mid link/claim flow. */
  | 'unclaimed-linkable'
  /** The server sees a Visitor: `me` resolved to null. */
  | 'anonymous'

export type Gate = 'page' | 'session-expired' | 'needs-invite'

/**
 * What `Layout` renders in the content area.
 *
 * The invariant: if the client believes there is a session, the server must
 * agree. When it does not — a rejected token, or `me` resolving to null while
 * the SPA still shows a login — we say so, instead of rendering a signed-in
 * shell over an anonymous session (which bottomed out in a contentless
 * "Something went wrong." on every player page).
 *
 * Public routes always render: a dead session must not lock a viewer out of
 * Rules/Schedule/Privacy, and `/invite/:code` is the way out of the invite
 * dead-end.
 */
export function contentGate(input: {
  access: Access
  sessionExpired: boolean
  /** The client believes a session exists (`label !== null`). */
  hasSession: boolean
  viewer: ViewerState
}): Gate {
  const { access, sessionExpired, hasSession, viewer } = input

  if (access === 'public') return 'page'
  if (sessionExpired) return 'session-expired'
  if (hasSession && viewer === 'anonymous') return 'session-expired'
  if (viewer === 'unclaimed') return 'needs-invite'
  return 'page'
}
```

- [ ] **Step 4: Run the test — it must pass**

Run: `cd web && npm test -- contentGate`
Expected: PASS, 8 tests.

- [ ] **Step 5: Commit**

```bash
git add web/src/auth/contentGate.ts web/src/auth/contentGate.test.ts
git commit -m "feat(web): add contentGate — the page/session-expired/needs-invite decision"
```

---

### Task 3: Detector 1 — a silent-refresh rejection marks the session expired

`resolveToken()` currently catches the Auth0 failure and returns `null`, so requests go out with **no** `Authorization` header and the server sees a Visitor. That silence is the bug.

**Files:**
- Modify: `web/src/auth/devAuth.ts` (the `resolveToken` function)
- Test: `web/src/auth/devAuth.test.ts` *(new)*

- [ ] **Step 1: Write the failing test**

```ts
// web/src/auth/devAuth.test.ts
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
```

- [ ] **Step 2: Run the test — it must fail**

Run: `cd web && npm test -- devAuth`
Expected: FAIL — the second test: `expected false to be true` (the failure is still swallowed).

- [ ] **Step 3: Implement**

In `web/src/auth/devAuth.ts`, add the import at the top of the file:

```ts
import { markSessionExpired } from './sessionState'
```

Then replace the `resolveToken` function:

```ts
/**
 * Resolve the bearer token for the current request.
 * - When Auth0 is active: calls `getAccessTokenSilently()` — cheap when the
 *   token is still valid, triggers a silent refresh when it has expired.
 * - Otherwise: reads from localStorage (dev-login / seeded token).
 *
 * A rejection means the refresh token is gone or revoked: the session can no
 * longer authenticate. We must NOT silently fall back to sending no token —
 * that lands the viewer in the server's Visitor state while the SPA still shows
 * them logged in, and every player page then renders a contentless error.
 */
export async function resolveToken(): Promise<string | null> {
  if (auth0Getter) {
    try {
      return await auth0Getter()
    } catch {
      markSessionExpired()
      return null
    }
  }
  return getToken()
}
```

- [ ] **Step 4: Run the test — it must pass**

Run: `cd web && npm test -- devAuth`
Expected: PASS, 2 tests.

- [ ] **Step 5: Commit**

```bash
git add web/src/auth/devAuth.ts web/src/auth/devAuth.test.ts
git commit -m "fix(web): a failed Auth0 silent refresh marks the session expired"
```

---

### Task 4: Detector 2 — a `401` from the auth seam marks the session expired

`crates/api/src/auth/seam.rs:41` answers `401 invalid token` for an expired/invalid JWT. `fetchWithAuth` is the one choke point every query **and** mutation passes through.

**Files:**
- Modify: `web/src/graphql/client.ts`
- Test: `web/src/graphql/client.test.ts` *(new)*

- [ ] **Step 1: Write the failing test**

```ts
// web/src/graphql/client.test.ts
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
    const spy = vi.fn(async () => new Response('{}', { status: 200 }))
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
```

- [ ] **Step 2: Run the test — it must fail**

Run: `cd web && npm test -- client`
Expected: FAIL — `fetchWithAuth` is not exported.

- [ ] **Step 3: Implement**

In `web/src/graphql/client.ts`, add the import:

```ts
import { markSessionExpired } from '../auth/sessionState'
```

Then export `fetchWithAuth` and handle the 401 (the rest of the file is unchanged):

```ts
/**
 * A custom fetch wrapper that injects a fresh bearer token on every request.
 *
 * urql v5 calls `fetchOptions` synchronously, so async token resolution must
 * happen here — inside the fetch itself — rather than in `fetchOptions`. The
 * Auth0 SDK caches the token internally and only hits the network when the
 * current token is near expiry, so per-request calls are cheap.
 *
 * Exported for unit tests. A `401` is the auth seam telling us the token is not
 * usable (`crates/api/src/auth/seam.rs`) — the single choke point where every
 * query and mutation learns the session is dead.
 */
export async function fetchWithAuth(
  input: RequestInfo | URL,
  init?: RequestInit,
): Promise<Response> {
  const token = await resolveToken()
  const devNow = getDevNow()

  const headers = new Headers(init?.headers)
  headers.set('content-type', 'application/json')
  if (token) headers.set('Authorization', `Bearer ${token}`)
  if (devNow) headers.set('X-Dev-Now', devNow)

  const response = await fetch(input, { ...init, headers })
  if (response.status === 401) markSessionExpired()
  return response
}
```

- [ ] **Step 4: Run the test — it must pass**

Run: `cd web && npm test -- client`
Expected: PASS, 2 tests.

- [ ] **Step 5: Commit**

```bash
git add web/src/graphql/client.ts web/src/graphql/client.test.ts
git commit -m "fix(web): a 401 from the auth seam marks the session expired"
```

---

### Task 5: `AuthState` — expose the flag, drop the stale label, add `reauthenticate()`

No unit test: these are React providers and the vitest environment is node-only (no jsdom). The behaviour is covered end-to-end in Task 9.

**Files:**
- Modify: `web/src/auth/authContextValue.ts`
- Modify: `web/src/auth/AuthContext.tsx`

- [ ] **Step 1: Widen `AuthState`**

Replace the whole of `web/src/auth/authContextValue.ts`:

```ts
import { createContext } from 'react'

export type AuthState = {
  /**
   * Display label for the currently-active player; null = visitor.
   *
   * Forced to null while `sessionExpired` — the server has rejected this
   * session, so the app must not keep claiming one (every `pause: !label`
   * query would otherwise keep firing against an anonymous session).
   */
  label: string | null
  /** The server no longer accepts this session — see `auth/sessionState.ts`. */
  sessionExpired: boolean
  login: (playerId: string) => Promise<void>
  logout: () => void
  /** Recover from a dead session: drop it, then start a fresh login. */
  reauthenticate: () => void
}

export const AuthContext = createContext<AuthState | null>(null)
```

- [ ] **Step 2: Update both providers**

In `web/src/auth/AuthContext.tsx`, update the imports:

```ts
import { useEffect, useMemo, useState, useSyncExternalStore, type ReactNode } from 'react'
import { useAuth0 } from '@auth0/auth0-react'
import { clearToken, devLogin as apiDevLogin, getDevPlayerLabel, setTokenFromAuth0 } from './devAuth'
import { auth0Enabled } from './auth0Provider'
import { AuthContext, type AuthState } from './authContextValue'
import {
  clearSessionExpired,
  isSessionExpired,
  markSessionExpired,
  subscribeSessionExpired,
} from './sessionState'
```

Replace `DevAuthProvider`:

```tsx
/** Dev-stub auth: the label is the seeded player chosen in the dev login bar. */
function DevAuthProvider({ children }: { children: ReactNode }) {
  const [player, setPlayer] = useState<string | null>(getDevPlayerLabel())
  const sessionExpired = useSyncExternalStore(subscribeSessionExpired, isSessionExpired)

  // Dropping the dead session is the same three moves for logout and for
  // re-login; in dev the "login" that follows is the auth-bar player picker,
  // which appears as soon as the label is null.
  const dropSession = () => {
    clearToken()
    clearSessionExpired()
    setPlayer(null)
  }

  const value = useMemo<AuthState>(
    () => ({
      label: sessionExpired ? null : player,
      sessionExpired,
      login: async (id: string) => {
        await apiDevLogin(id)
        clearSessionExpired()
        setPlayer(id)
      },
      logout: dropSession,
      reauthenticate: dropSession,
    }),
    // `dropSession` is re-created every render but closes over only setState,
    // which is stable — no need to re-memo on it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [player, sessionExpired],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}
```

Replace `Auth0AuthProvider`:

```tsx
/**
 * Auth0 auth. `label` becomes truthy ONLY once the access token has actually
 * been fetched — not merely when `isAuthenticated` flips. Auth0 access tokens
 * omit `email`, so the API resolves it from `/userinfo`; firing `me` before the
 * token is attached would resolve to a Visitor and (via the document cache)
 * stick. Gating on a fetched token guarantees every `pause: !label` query goes
 * out with a bearer.
 *
 * When the silent fetch FAILS the refresh token is gone or revoked. The SDK
 * keeps a cached `user` in localStorage that outlives it, so `isAuthenticated`
 * stays true — which is how the app used to render a signed-in shell over a
 * session the server rejects. Marking the session expired is what stops that.
 */
function Auth0AuthProvider({ children }: { children: ReactNode }) {
  const { isAuthenticated, user, getAccessTokenSilently, loginWithRedirect, logout } = useAuth0()
  const [tokenReady, setTokenReady] = useState(false)
  const sessionExpired = useSyncExternalStore(subscribeSessionExpired, isSessionExpired)

  useEffect(() => {
    if (!isAuthenticated) return
    let active = true
    void getAccessTokenSilently()
      .then((token) => {
        setTokenFromAuth0(token)
        clearSessionExpired()
        if (active) setTokenReady(true)
      })
      .catch(() => {
        // No usable token. Drop the stale one and surface the dead session —
        // unblocking `tokenReady` so the app renders the SessionExpired view
        // rather than hanging on a spinner.
        clearToken()
        markSessionExpired()
        if (active) setTokenReady(true)
      })
    // Reset on logout / principal change in cleanup (not synchronously in the
    // body) so a re-login re-gates on a freshly fetched token.
    return () => {
      active = false
      setTokenReady(false)
    }
  }, [isAuthenticated, getAccessTokenSilently])

  const label =
    isAuthenticated && tokenReady && !sessionExpired
      ? (user?.email ?? user?.name ?? 'player')
      : null

  const value = useMemo<AuthState>(
    () => ({
      label,
      sessionExpired,
      // Auth0 login/logout is normally driven from ProdAuthBar via the SDK
      // directly; these keep `useAuth()` consistent for any other caller.
      login: async () => {
        await loginWithRedirect()
      },
      logout: () => {
        clearToken()
        clearSessionExpired()
        logout({ logoutParams: { returnTo: window.location.origin } })
      },
      // Straight back into Auth0, returning to the page they were on. If the
      // tenant session is still alive this is a silent round-trip; if not, they
      // land on the login form.
      reauthenticate: () => {
        clearToken()
        clearSessionExpired()
        void loginWithRedirect({
          appState: { returnTo: window.location.pathname + window.location.search },
        })
      },
    }),
    [label, sessionExpired, loginWithRedirect, logout],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}
```

- [ ] **Step 3: Typecheck**

Run: `cd web && npm run build`
Expected: PASS. (`tsc -b` proves every `AuthState` construction site now supplies `sessionExpired` and `reauthenticate`.)

- [ ] **Step 4: Commit**

```bash
git add web/src/auth/authContextValue.ts web/src/auth/AuthContext.tsx
git commit -m "feat(web): AuthState exposes sessionExpired and reauthenticate()"
```

---

### Task 6: Strings + the `SessionExpired` view

**Files:**
- Modify: `web/src/i18n/strings.ts` (English block ~line 107; Hungarian block ~line 491)
- Create: `web/src/components/SessionExpired.tsx`
- Modify: `web/src/index.css` (next to `.needs-invite`, ~line 1087)

- [ ] **Step 1: Add the English strings**

In `web/src/i18n/strings.ts`, after `notAdminBody: 'This screen is for admins only.',` add:

```ts
  // expired session (the server rejected the token / `me` came back null)
  sessionExpiredTitle: 'Your session has expired',
  sessionExpiredBody:
    'You have been signed out. Log in again to see your tips — nothing you saved is lost.',
  logInAgain: 'Log in again',
```

- [ ] **Step 2: Add the Hungarian strings**

After `notAdminBody: 'Ez az oldal csak adminoknak.',` add:

```ts
  // lejárt munkamenet (a szerver elutasította a tokent / a `me` null lett)
  sessionExpiredTitle: 'A munkameneted lejárt',
  sessionExpiredBody:
    'Kiléptettünk. Lépj be újra, hogy lásd a tippjeidet — semmi nem veszett el, amit elmentettél.',
  logInAgain: 'Belépés újra',
```

- [ ] **Step 3: Create the view**

```tsx
// web/src/components/SessionExpired.tsx
import { useI18n } from '../i18n/useI18n'
import { useAuth } from '../auth/useAuth'

/**
 * The dead-end for a viewer whose session the server no longer accepts: an
 * expired/revoked Auth0 refresh token, a 401 from the auth seam, or `me`
 * resolving to null while the SPA still shows a login.
 *
 * Rendered in the content area in place of a player- or admin-only page, the
 * same way `NeedsInvite` is; public pages stay reachable (see `contentGate`).
 * Before this existed, that state rendered a bare `ErrorView` — a contentless
 * "Something went wrong." that told the player nothing and left them stuck.
 */
export function SessionExpired() {
  const { t } = useI18n()
  const { reauthenticate } = useAuth()

  return (
    <div className="status session-expired">
      <h2>{t('sessionExpiredTitle')}</h2>
      <p>{t('sessionExpiredBody')}</p>
      <button type="button" onClick={reauthenticate}>
        {t('logInAgain')}
      </button>
    </div>
  )
}
```

- [ ] **Step 4: Style it (a new class name with no CSS renders unstyled)**

In `web/src/index.css`, directly after the `.needs-invite` rules (~line 1089):

```css
.session-expired { max-width: 40rem; }
.session-expired h2 { color: var(--amber-bright); margin: 0 0 8px; }
.session-expired p { color: var(--text-dim); margin: 0 0 12px; line-height: 1.5; }
```

- [ ] **Step 5: Typecheck**

Run: `cd web && npm run build`
Expected: PASS. (The `Strings` type is `typeof en`, so a key missing from the Hungarian block fails the build — proof both locales are covered.)

- [ ] **Step 6: Commit**

```bash
git add web/src/i18n/strings.ts web/src/components/SessionExpired.tsx web/src/index.css
git commit -m "feat(web): add the SessionExpired view (en + hu)"
```

---

### Task 7: Wire `Layout` — detector 3, and render the view

**Files:**
- Modify: `web/src/components/Layout.tsx`

- [ ] **Step 1: Update the imports**

```ts
import { useAuth } from '../auth/useAuth'
import { contentGate, type ViewerState } from '../auth/contentGate'
import { SessionExpired } from './SessionExpired'
```

(`accessFor` is already imported; keep it.)

- [ ] **Step 2: Replace the derivation block**

Replace these lines in `Layout()`:

```tsx
  const meRaw = meResult.data?.me ?? null
  const me = meRaw?.__typename === 'Player' ? meRaw : null
  const isUnclaimed =
    meRaw?.__typename === 'UnclaimedViewer' && !meRaw.linkCandidate
  // Optimistic player-nav signal: ...
  const showPlayerNav = Boolean(label) && !isUnclaimed
  const deadEnd = isUnclaimed && accessFor(location.pathname) !== 'public'
```

with:

```tsx
  const { label, sessionExpired } = useAuth()   // widen the existing useAuth() call
```

```tsx
  const meRaw = meResult.data?.me ?? null
  const me = meRaw?.__typename === 'Player' ? meRaw : null

  // `data === undefined` means the query is still in flight or paused — NOT
  // that the server said Visitor. Only an explicit null `me` is anonymous;
  // conflating the two would flash the session-expired view on every load.
  const viewer: ViewerState =
    meResult.data === undefined
      ? 'loading'
      : meRaw === null
        ? 'anonymous'
        : meRaw.__typename === 'Player'
          ? 'player'
          : meRaw.linkCandidate
            ? 'unclaimed-linkable'
            : 'unclaimed'

  const gate = contentGate({
    access: accessFor(location.pathname),
    sessionExpired,
    hasSession: Boolean(label),
    viewer,
  })

  // Optimistic player-nav signal: show player links as soon as a session
  // exists, hiding them only once the viewer is *confirmed* unclaimed. This
  // keeps nav synchronous for a real player (no flash-of-hidden-nav while the
  // `me` query is in flight) and still hides it at the invite dead-end.
  const showPlayerNav = Boolean(label) && viewer !== 'unclaimed'
```

- [ ] **Step 3: Replace the content slot**

Replace:

```tsx
      <main className="content">
        {deadEnd ? <NeedsInvite /> : <Outlet />}
      </main>
```

with:

```tsx
      <main className="content">
        {gate === 'session-expired' ? (
          <SessionExpired />
        ) : gate === 'needs-invite' ? (
          <NeedsInvite />
        ) : (
          <Outlet />
        )}
      </main>
```

- [ ] **Step 4: Update the component doc-comment**

Extend the existing block comment above `export function Layout()` with:

```
 * Dead session: when the server no longer accepts the token (or `me` comes back
 * null while the SPA still shows a login), `SessionExpired` replaces the page on
 * every non-public route. See `auth/contentGate.ts`.
```

- [ ] **Step 5: Typecheck + lint**

Run: `cd web && npm run build && npm run lint`
Expected: both PASS.

- [ ] **Step 6: Commit**

```bash
git add web/src/components/Layout.tsx
git commit -m "feat(web): Layout renders SessionExpired for a dead session"
```

---

### Task 8: `ProdAuthBar` must respect the dead session

`ProdAuthBar` keys off Auth0's `isAuthenticated`, **not** `label` — so without this it keeps rendering "Logged in as &lt;their email&gt;" next to a "your session expired" page. That contradiction is half of what made the original bug so confusing.

**Files:**
- Modify: `web/src/components/AuthBar.tsx` (`ProdAuthBar`, lines ~28-57)

- [ ] **Step 1: Update the imports**

```ts
import { useAuth } from '../auth/useAuth'
import { clearToken } from '../auth/devAuth'
import { clearSessionExpired } from '../auth/sessionState'
```

(`useAuth` and `clearToken` are already imported; add `clearSessionExpired`.)

- [ ] **Step 2: Gate the signed-in branch on a live session**

In `ProdAuthBar`, replace:

```tsx
  const { isAuthenticated, loginWithRedirect, logout, user } = useAuth0()
  const { t } = useI18n()
```

with:

```tsx
  const { isAuthenticated, loginWithRedirect, logout, user } = useAuth0()
  const { t } = useI18n()
  const { sessionExpired } = useAuth()
  // The SDK keeps a cached `user` that outlives the refresh token, so
  // `isAuthenticated` alone would keep showing "Logged in as …" over a session
  // the server has already rejected.
  const signedIn = isAuthenticated && !sessionExpired
```

Then change the `me` query's pause and the signed-out branch's guard:

```tsx
  const [meResult] = useQuery<{ me: Me }>({
    query: ME_QUERY,
    pause: !signedIn,
  })
  const meRaw = meResult.data?.me
  const me = meRaw?.__typename === 'Player' ? meRaw : null
  if (!signedIn) {
```

- [ ] **Step 3: Clear the dead session before the front-door login**

In that same signed-out branch, replace the login button's `onClick`:

```tsx
          onClick={() => {
            // A viewer arriving here from an expired session still has the dead
            // token in localStorage; drop it so the fresh login is not racing a
            // stale bearer.
            clearToken()
            clearSessionExpired()
            void loginWithRedirect({
              appState: { returnTo: window.location.pathname + window.location.search },
              authorizationParams: { screen_hint: 'login' },
            })
          }}
```

- [ ] **Step 4: Typecheck + lint**

Run: `cd web && npm run build && npm run lint`
Expected: both PASS.

- [ ] **Step 5: Commit**

```bash
git add web/src/components/AuthBar.tsx
git commit -m "fix(web): the prod auth bar stops claiming a rejected session"
```

---

### Task 9: End-to-end — a rejected token shows the view, not "Something went wrong."

This drives **detector 2** against the real API: the seam really returns 401, the real SPA really renders the view. Dev-stub mode, so no Auth0 tenant is needed.

**Files:**
- Create: `web/e2e/session-expired.spec.ts`

- [ ] **Step 1: Write the failing e2e test**

```ts
// web/e2e/session-expired.spec.ts
import { expect, test } from '@playwright/test'
import { devLogin } from './helpers'

/**
 * The production bug: a player's token stopped being accepted (an expired Auth0
 * refresh token), but the SPA kept the cached label and rendered a signed-in
 * shell over an anonymous session — so every player page showed a bare
 * "Something went wrong." with no way forward.
 *
 * Reproduced here by keeping the dev label while swapping the JWT for one the
 * auth seam rejects (401) — the same end state, exercised on the real wire.
 */
const REJECTED_JWT =
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJnb25lIn0.this-signature-is-not-valid'

async function poisonTheToken(page: import('@playwright/test').Page) {
  await page.evaluate((jwt) => {
    // Keep `xpool.devPlayer` — that is what makes the SPA still believe it has
    // a session. Only the credential is dead.
    localStorage.setItem('xpool.jwt', jwt)
  }, REJECTED_JWT)
}

test('a rejected token shows the session-expired view, not a bare error', async ({ page }) => {
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  await poisonTheToken(page)
  await page.goto('/mytips')

  await expect(page.getByRole('heading', { name: 'Your session has expired' })).toBeVisible()
  await expect(page.getByRole('button', { name: 'Log in again' })).toBeVisible()

  // The symptom that started this must be gone.
  await expect(page.locator('.status-error')).toHaveCount(0)
  await expect(page.getByText('Something went wrong')).toHaveCount(0)
})

test('public pages stay reachable with a dead session', async ({ page }) => {
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  await poisonTheToken(page)
  await page.goto('/rules')

  await expect(page.getByText('Your session has expired')).toHaveCount(0)
  await expect(page.locator('.page')).toBeVisible()
})

test('"Log in again" clears the dead session and offers a fresh login', async ({ page }) => {
  await page.goto('/')
  await devLogin(page, 'demo-ada')

  await poisonTheToken(page)
  await page.goto('/mytips')

  await page.getByRole('button', { name: 'Log in again' }).click()

  // In dev-stub mode the way back in is the auth-bar player picker.
  await expect(page.locator('.auth-bar')).toContainText('You are outside.')
  await expect(page.locator('.auth-picker select')).toBeVisible()
  expect(await page.evaluate(() => localStorage.getItem('xpool.jwt'))).toBeNull()
})
```

- [ ] **Step 2: Run it — it must fail against the pre-fix behaviour**

If you are implementing tasks in order, the fix is already in, so this passes immediately. To *prove* the test is real, stash the fix once and watch it fail:

Run: `cd web && npm run e2e -- session-expired`
Expected (pre-fix): FAIL — the page shows "Something went wrong." and no heading.
Expected (post-fix): PASS, 3 tests.

- [ ] **Step 3: Run the whole e2e suite (no regressions in the invite dead-end / auth flows)**

Run: `cd web && npm run e2e`
Expected: PASS. Pay attention to `auth.spec.ts`, `invite-entry.spec.ts`, and `onboarding.spec.ts` — they exercise the same Layout gate.

- [ ] **Step 4: Commit**

```bash
git add web/e2e/session-expired.spec.ts
git commit -m "test(web): e2e for the expired-session dead-end"
```

---

### Task 10: Full verification + a real look at the page

Green typecheck/lint/e2e is not proof it *looks* right — check the rendered page.

- [ ] **Step 1: The full gate**

```bash
cd web && npm test && npm run build && npm run lint && npm run e2e
```

Expected: all PASS. Paste the actual output into the summary; do not claim success from memory.

- [ ] **Step 2: Look at it**

With the dev stack running (`bin/local-dev`), log in as `demo-ada` at `:5173`, then in DevTools:

```js
localStorage.setItem('xpool.jwt', 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJnb25lIn0.nope')
```

Reload `/mytips`. Confirm: the heading and body render *styled* (not unstyled text), the button is visible and clickable, the auth bar no longer claims a login, and the player nav is gone. Switch the language to Hungarian and confirm the Hungarian strings render.

- [ ] **Step 3: Merge**

Follow `superpowers:finishing-a-development-branch` — merge the branch into `master` locally (no history rewriting), then push.

---

## Traceability against the spec

| Spec requirement | Task |
|---|---|
| Detector 1 — silent-refresh rejection | 3 |
| Detector 2 — 401 from the seam | 4 |
| Detector 3 — `me` null while the client claims a session | 2 (logic) + 7 (wiring) |
| One `sessionExpired` state | 1 |
| `SessionExpired` view, sibling of `NeedsInvite` | 6 |
| Rendered for non-public routes only | 2 + 7 |
| Stale `label` dropped / auth bar stops lying | 5 + 8 |
| `reauthenticate()` — provider-agnostic recovery | 5 |
| en + hu strings | 6 |
| E2E driving detector 2 | 9 |
| Invite dead-end regression guard | 2 (`contentGate` tests) + 9 (suite run) |

**Deviation from the spec, deliberate:** the spec called for a *component* test of detector 3. The vitest setup is node-environment with no jsdom or testing-library, and adding a React-rendering test harness is out of scope for a bug fix. Detector 3's decision is therefore extracted as the pure `contentGate()` function and unit-tested there (Task 2), with `Layout` reduced to deriving its inputs. The invite dead-end regression guard the spec asked for is preserved in those same tests.

**Out of scope (unchanged from the spec):** the other bare `<ErrorView />` call sites, including `MyTipsPage.tsx:255`, which stays reachable for an authenticated-but-unclaimed viewer who *has* a link candidate.
