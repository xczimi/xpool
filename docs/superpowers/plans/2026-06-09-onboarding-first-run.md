# Onboarding & First-Run UX Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the four first-run problems — confusing "log in", invite code lost across the Auth0 redirect, an unusable claim form, and a hard-to-notice settings gear.

**Architecture:** Web-only (no API/schema change). A unit-testable `returnTo` handoff preserves the invite path across the Auth0 redirect; a shared `NameForm` unifies the claim and Profile forms; `InviteClaimPage` is rewritten as the i18n'd onboarding hub with a "Continue to join" entry; the settings gear gets a visible label.

**Tech Stack:** React + TypeScript + Vite, `@auth0/auth0-react@2.17.0`, urql, react-router-dom, Vitest (unit), Playwright (e2e).

**Spec:** `docs/superpowers/specs/2026-06-09-onboarding-first-run-design.md`

---

## Key facts the implementer must know

- **`Auth0Gate` is the outermost provider, OUTSIDE `BrowserRouter`** (`web/src/main.tsx`). So the redirect-restore cannot use react-router `navigate` from `onRedirectCallback`; the path is stashed (sessionStorage) by `onRedirectCallback` and consumed by an in-Router `<PostLoginRedirect>`.
- **`useAuth0()` is safe to call without a provider** (dev/e2e mode, `auth0Enabled === false`): it returns a stub context whose methods throw only if *invoked*. So `InviteClaimPage` may call `useAuth0()` unconditionally and render in both modes; the click handler invokes `loginWithRedirect` only when `auth0Enabled`.
- **Vitest runs in the `node` env (no `jsdom`)** — there is no global `sessionStorage` in unit tests. `returnTo.ts` therefore takes an injectable `Storage` (default `globalThis.sessionStorage`); the unit test passes a fake.
- **`web/e2e/invite-entry.spec.ts:27` asserts the OLD copy** `'Log in to claim this invite.'`. Part A removes that branch — this assertion MUST be updated or the suite breaks.
- E2e runs in **dev mode** (Auth0 disabled). Prod-only UI (the real Auth0 button behaviour, the `ProdAuthBar` header wording) can't be exercised there; e2e asserts what's mode-agnostic (the invite page renders "Continue to join", the gear shows a "Settings" label, the Profile form renders). The `returnTo` handoff is covered by the unit test.
- The worktree needs `web/.env.local` (gitignored) blanking `VITE_AUTH0_*` so the dev-login bar renders in e2e — copy it from the main checkout if absent.

## Files

- **Create** `web/src/auth/returnTo.ts` + `web/src/auth/returnTo.test.ts`
- **Create** `web/src/components/PostLoginRedirect.tsx`
- **Create** `web/src/components/NameForm.tsx`
- **Modify** `web/src/auth/auth0Provider.tsx` — `onRedirectCallback` stashes `appState.returnTo`
- **Modify** `web/src/App.tsx` — render `<PostLoginRedirect/>` inside the Router
- **Modify** `web/src/pages/ProfilePage.tsx` — `ProfileForm` renders `NameForm`
- **Modify** `web/src/pages/InviteClaimPage.tsx` — full rewrite (i18n, Continue-to-join, NameForm, `useNavigate`)
- **Modify** `web/src/components/AuthBar.tsx` — header login passes `appState.returnTo`
- **Modify** `web/src/components/SettingsMenu.tsx` — visible "Settings" label
- **Modify** `web/src/index.css` — `.settings-gear` bigger/brighter/inline label
- **Modify** `web/src/i18n/strings.ts` — change `frontDoorMembers`; add invite/join/link keys (EN + HU)
- **Modify** `web/e2e/invite-entry.spec.ts` — update the stale assertion
- **Create** `web/e2e/onboarding.spec.ts` — settings label + Profile form regression

---

## Task 0: Branch

Web source must go on a branch/worktree (CLAUDE.md "Branch discipline").

- [ ] **Step 1: Create the worktree**

```bash
cd /Users/xczimi/Private/SoccerPool/xpool
git worktree add .claude/worktrees/onboarding-first-run -b onboarding-first-run
```

(If executing via the worktree skill, that already satisfies this.) All subsequent paths are relative to the worktree's `web/`.

- [ ] **Step 2: Ensure dev-stub auth env exists (for e2e later)**

```bash
test -f web/.env.local || cp /Users/xczimi/Private/SoccerPool/xpool/web/.env.local web/.env.local
```

---

## Task 1: `returnTo` handoff helper (Part A core)

**Files:**
- Create: `web/src/auth/returnTo.ts`
- Test: `web/src/auth/returnTo.test.ts`

- [ ] **Step 1: Write the failing test**

Create `web/src/auth/returnTo.test.ts`:

```ts
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
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd web && npm test -- returnTo`
Expected: FAIL — `No "stashReturnTo" export is defined`.

- [ ] **Step 3: Implement the helper**

Create `web/src/auth/returnTo.ts`:

```ts
/**
 * One-shot handoff for the path to return to after an Auth0 sign-in redirect.
 *
 * The Auth0 redirect lands the app back on `/` (the SDK `redirect_uri` is the
 * origin), so the page the user started from — e.g. their `/invite/<code>` — is
 * otherwise lost. `onRedirectCallback` stashes the path here; `PostLoginRedirect`
 * (inside the Router) takes it and navigates. Backed by `sessionStorage` so it
 * survives the redirect round-trip in the same tab. `Storage` is injectable so
 * the helper is unit-testable without a browser (vitest runs in the node env).
 */
const KEY = 'xpool.returnTo'

export function stashReturnTo(
  path: string,
  storage: Storage = globalThis.sessionStorage,
): void {
  storage.setItem(KEY, path)
}

/** Read-and-clear the stashed path (returns null if none). */
export function takeReturnTo(
  storage: Storage = globalThis.sessionStorage,
): string | null {
  const value = storage.getItem(KEY)
  if (value !== null) storage.removeItem(KEY)
  return value
}
```

- [ ] **Step 4: Run it to verify it passes**

Run: `cd web && npm test -- returnTo`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add web/src/auth/returnTo.ts web/src/auth/returnTo.test.ts
git commit -m "feat(web): returnTo handoff helper for preserving the invite path across Auth0"
```

---

## Task 2: Restore the path after the Auth0 redirect (Part A wiring)

**Files:**
- Modify: `web/src/auth/auth0Provider.tsx`
- Create: `web/src/components/PostLoginRedirect.tsx`
- Modify: `web/src/App.tsx`

- [ ] **Step 1: Stash `returnTo` in `onRedirectCallback`**

In `web/src/auth/auth0Provider.tsx`, add the import after line 4:

```ts
import { stashReturnTo } from './returnTo'
```

Then add an `onRedirectCallback` prop to `<SdkProvider>` (between `clientId` and `authorizationParams`):

```tsx
    <SdkProvider
      domain={DOMAIN!}
      clientId={CLIENT!}
      onRedirectCallback={(appState) => {
        const returnTo = (appState as { returnTo?: string } | undefined)?.returnTo
        if (returnTo) stashReturnTo(returnTo)
      }}
      authorizationParams={{
        redirect_uri: window.location.origin,
        audience: AUDIENCE,
        scope: 'openid profile email offline_access',
      }}
      cacheLocation="localstorage"
      useRefreshTokens
    >
```

- [ ] **Step 2: Create the in-Router redirect consumer**

Create `web/src/components/PostLoginRedirect.tsx`:

```tsx
import { useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth0 } from '@auth0/auth0-react'
import { takeReturnTo } from '../auth/returnTo'

/**
 * After an Auth0 sign-in redirect lands the app back on `/`, restore the page
 * the user started from (e.g. their `/invite/<code>`), which `onRedirectCallback`
 * stashed via `stashReturnTo`. Gated on `isAuthenticated` so it runs only once
 * the SDK has processed the redirect (which is after the stash), avoiding a race
 * with this component's mount. No-op in dev (no Auth0 provider → `isAuthenticated`
 * stays false, and nothing is ever stashed).
 */
export function PostLoginRedirect() {
  const navigate = useNavigate()
  const { isAuthenticated } = useAuth0()
  useEffect(() => {
    if (!isAuthenticated) return
    const target = takeReturnTo()
    if (target && target !== window.location.pathname) {
      navigate(target, { replace: true })
    }
  }, [isAuthenticated, navigate])
  return null
}
```

- [ ] **Step 3: Render it inside the Router**

In `web/src/App.tsx`, add the import after line 2 (`import { Layout }...`):

```ts
import { PostLoginRedirect } from './components/PostLoginRedirect'
```

Wrap the returned `<Routes>` in a fragment with `<PostLoginRedirect/>` first. Change:

```tsx
  return (
    <Routes>
      <Route element={<Layout />}>
```

to:

```tsx
  return (
    <>
      <PostLoginRedirect />
      <Routes>
        <Route element={<Layout />}>
```

…and close the fragment: change the end of the component from:

```tsx
      </Route>
    </Routes>
  )
}
```

to:

```tsx
      </Route>
      </Routes>
    </>
  )
}
```

- [ ] **Step 4: Verify types + lint**

Run: `cd web && npx tsc -b --noEmit && npm run lint`
Expected: both exit 0.

- [ ] **Step 5: Commit**

```bash
git add web/src/auth/auth0Provider.tsx web/src/components/PostLoginRedirect.tsx web/src/App.tsx
git commit -m "feat(web): restore the start path after the Auth0 sign-in redirect"
```

---

## Task 3: Shared `NameForm` + Profile refactor (Part B)

**Files:**
- Create: `web/src/components/NameForm.tsx`
- Modify: `web/src/pages/ProfilePage.tsx`

- [ ] **Step 1: Create the shared form**

Create `web/src/components/NameForm.tsx`:

```tsx
import { useState } from 'react'
import type { FormEvent } from 'react'
import { useI18n } from '../i18n/useI18n'

/**
 * The nick + full-name form, shared by the first-run claim step and the Profile
 * page so they look and behave identically (the Profile form was the good one).
 * Presentational: it owns the field state and calls `onSubmit`; the parent owns
 * the mutation and the `flash` message. Submit is disabled while `busy` or when
 * the nick is empty.
 */
export function NameForm({
  initialNick = '',
  initialFullName = '',
  submitLabel,
  busy = false,
  flash = null,
  onSubmit,
}: {
  initialNick?: string
  initialFullName?: string
  submitLabel: string
  busy?: boolean
  flash?: string | null
  onSubmit: (nick: string, fullName: string) => void
}) {
  const { t } = useI18n()
  const [nick, setNick] = useState(initialNick)
  const [fullName, setFullName] = useState(initialFullName)
  const submit = (e: FormEvent) => {
    e.preventDefault()
    onSubmit(nick, fullName)
  }
  return (
    <>
      {flash && <p className="flash-bar">{flash}</p>}
      <form className="form" onSubmit={submit}>
        <label>
          {t('nick')}
          <input value={nick} onChange={(e) => setNick(e.target.value)} />
        </label>
        <label>
          {t('fullName')}
          <input value={fullName} onChange={(e) => setFullName(e.target.value)} />
        </label>
        <button type="submit" className="primary" disabled={busy || !nick.trim()}>
          {submitLabel}
        </button>
      </form>
    </>
  )
}
```

- [ ] **Step 2: Refactor `ProfileForm` to render `NameForm`**

In `web/src/pages/ProfilePage.tsx`, add the import after the `StatusViews` import:

```ts
import { NameForm } from '../components/NameForm'
```

Replace the entire `ProfileForm` function (the `function ProfileForm({ me }: { me: Player }) { ... }` block) with:

```tsx
function ProfileForm({ me }: { me: Player }) {
  const { t } = useI18n()
  const [updateState, updateProfile] = useMutation(UPDATE_PROFILE_MUTATION)
  const [flash, setFlash] = useState<string | null>(null)
  return (
    <NameForm
      initialNick={me.nick}
      initialFullName={me.fullName}
      submitLabel={t('save')}
      busy={updateState.fetching}
      flash={flash}
      onSubmit={async (nick, fullName) => {
        setFlash(null)
        const res = await updateProfile({ nick, fullName })
        setFlash(res.error ? `${t('errorPrefix')}: ${res.error.message}` : t('profileSaved'))
      }}
    />
  )
}
```

(`ProfilePage` keeps `<ProfileForm key={me.id} me={me} />` — the `key` remounts the form, and thus `NameForm`, when the loaded player changes, so the fields re-seed.)

- [ ] **Step 3: Verify types + lint**

Run: `cd web && npx tsc -b --noEmit && npm run lint`
Expected: both exit 0. (If lint flags an unused `React`/`FormEvent` import in `ProfilePage`, remove it — `ProfileForm` no longer builds a `FormEvent` handler itself.)

- [ ] **Step 4: Commit**

```bash
git add web/src/components/NameForm.tsx web/src/pages/ProfilePage.tsx
git commit -m "feat(web): extract shared NameForm; Profile renders it"
```

---

## Task 4: i18n keys + header login wording/returnTo (Parts A, C)

**Files:**
- Modify: `web/src/i18n/strings.ts`
- Modify: `web/src/components/AuthBar.tsx`

- [ ] **Step 1: Change `frontDoorMembers` + add keys in the `en` catalogue**

In `web/src/i18n/strings.ts`, find the EN `frontDoorMembers` line (currently `frontDoorMembers: 'Members: log in',`) and change it to:

```ts
  frontDoorMembers: 'Already playing? Log in',
```

Then, in the EN catalogue, immediately after the `rulesTitle: 'Rules & Scoring',` line, add:

```ts
  // onboarding / invite claim
  inviteWelcomeTitle: "You've been invited to xPool!",
  inviteWelcomeBody:
    "We'll set up a quick, secure sign-in (email or Google) so only you can enter your picks.",
  inviteContinue: 'Continue to join',
  inviteClaimTitle: 'Accept your invite',
  inviteClaimBody: 'Set your display name.',
  join: 'Join',
  inviteJoinTitle: 'Join this pool',
  inviteJoinBody: 'Accept this invite to join the pool.',
  inviteJoinedPrefix: "You're in",
  inviteGoScoreboard: 'Go to the scoreboard',
  inviteLinkTitle: 'Link this login?',
  inviteLinkBody: 'An account already exists for this email. Link this login to it?',
  inviteLinkConfirm: 'Yes, link',
  inviteLinkCancel: 'No, cancel',
  inviteMissingCode: 'Missing invite code.',
```

- [ ] **Step 2: Mirror into the `hu` catalogue**

Find the HU `frontDoorMembers` line and change it to:

```ts
  frontDoorMembers: 'Már játszol? Belépés',
```

Then, in the HU catalogue, immediately after the `rulesTitle: 'Szabályok és pontozás',` line, add:

```ts
  inviteWelcomeTitle: 'Meghívtak az xPoolba!',
  inviteWelcomeBody:
    'Beállítunk egy gyors, biztonságos belépést (e-mail vagy Google), hogy csak te adhasd meg a tippjeidet.',
  inviteContinue: 'Tovább a csatlakozáshoz',
  inviteClaimTitle: 'Fogadd el a meghívót',
  inviteClaimBody: 'Add meg a megjelenítendő neved.',
  join: 'Csatlakozás',
  inviteJoinTitle: 'Csatlakozz ehhez a tutihoz',
  inviteJoinBody: 'Fogadd el a meghívót, hogy csatlakozz a tutihoz.',
  inviteJoinedPrefix: 'Bent vagy itt:',
  inviteGoScoreboard: 'Irány az eredménytábla',
  inviteLinkTitle: 'Összekapcsolod ezt a belépést?',
  inviteLinkBody: 'Már létezik fiók ehhez az e-mailhez. Összekapcsolod vele ezt a belépést?',
  inviteLinkConfirm: 'Igen, kapcsold össze',
  inviteLinkCancel: 'Nem, mégse',
  inviteMissingCode: 'Hiányzó meghívókód.',
```

- [ ] **Step 3: Header login passes `appState.returnTo`**

In `web/src/components/AuthBar.tsx`, change the logged-out `loginWithRedirect` call (in `ProdAuthBar`) from:

```tsx
          onClick={() =>
            void loginWithRedirect({ authorizationParams: { screen_hint: 'login' } })
          }
```

to:

```tsx
          onClick={() =>
            void loginWithRedirect({
              appState: { returnTo: window.location.pathname + window.location.search },
              authorizationParams: { screen_hint: 'login' },
            })
          }
```

- [ ] **Step 4: Verify types, lint, unit (i18n balance test must stay green)**

Run: `cd web && npx tsc -b --noEmit && npm run lint && npm test`
Expected: all pass — both catalogues have the same keys, so the i18n balance test stays green.

- [ ] **Step 5: Commit**

```bash
git add web/src/i18n/strings.ts web/src/components/AuthBar.tsx
git commit -m "feat(web): onboarding i18n keys; clearer header login wording + returnTo"
```

---

## Task 5: Rewrite `InviteClaimPage` (Parts A + B)

**Files:**
- Modify: `web/src/pages/InviteClaimPage.tsx` (full rewrite)

- [ ] **Step 1: Replace the whole file**

Replace the entire contents of `web/src/pages/InviteClaimPage.tsx` with:

```tsx
import { Link, useNavigate, useParams } from 'react-router-dom'
import { useMutation, useQuery } from 'urql'
import { useAuth0 } from '@auth0/auth0-react'
import { useI18n } from '../i18n/useI18n'
import { auth0Enabled } from '../auth/auth0Provider'
import { NameForm } from '../components/NameForm'

const ME = `query ViewerState {
  me {
    __typename
    ... on Player { id nick }
    ... on UnclaimedViewer {
      email
      linkCandidate { personId provider }
    }
  }
}`

const CLAIM = `mutation Claim($code: String!, $nick: String!, $fullName: String!) {
  claimInvite(code: $code, nick: $nick, fullName: $fullName) { player { id nick } }
}`

const JOIN = `mutation Join($code: String!) {
  join(code: $code) { id name }
}`

const LINK = `mutation Link($personId: String!) {
  confirmLink(personId: $personId) { player { id } }
}`

type PlayerViewer = { __typename: 'Player'; id: string; nick: string }
type UnclaimedViewerShape = {
  __typename: 'UnclaimedViewer'
  email?: string | null
  linkCandidate?: { personId: string; provider: string } | null
}
type ViewerShape = PlayerViewer | UnclaimedViewerShape

/**
 * The invite link is the front door to identity. This page handles every state
 * a recipient can be in:
 *  - logged out      → welcome + "Continue to join" (Auth0, preserving the code)
 *  - already a Player → accept the invite (join the pool)
 *  - unclaimed + a matching account → offer to link the login (AUTH-13)
 *  - unclaimed, new   → set a display name (shared NameForm) and claim
 */
export function InviteClaimPage() {
  const { t } = useI18n()
  const { code } = useParams<{ code: string }>()
  const navigate = useNavigate()
  const { loginWithRedirect } = useAuth0()
  const [meResult] = useQuery({ query: ME })
  const [claimResult, runClaim] = useMutation(CLAIM)
  const [joinResult, runJoin] = useMutation(JOIN)
  const [linkResult, runLink] = useMutation(LINK)

  if (!code)
    return (
      <main className="content">
        <p>{t('inviteMissingCode')}</p>
      </main>
    )
  if (meResult.fetching) return null

  const viewer = meResult.data?.me as ViewerShape | null | undefined

  // Logged out — establish identity, preserving this invite path across Auth0.
  if (!viewer) {
    const onContinue = () => {
      const returnTo = `/invite/${code}`
      if (auth0Enabled) {
        void loginWithRedirect({
          appState: { returnTo },
          authorizationParams: { screen_hint: 'signup' },
        })
      } else {
        // Dev/e2e: no Auth0 — the auth bar's player picker is the sign-in.
        navigate('/')
      }
    }
    return (
      <main className="content">
        <h2>{t('inviteWelcomeTitle')}</h2>
        <p>{t('inviteWelcomeBody')}</p>
        <button type="button" className="primary" onClick={onContinue}>
          {t('inviteContinue')}
        </button>
      </main>
    )
  }

  // Already a Player — accept the invite (join the pool).
  if (viewer.__typename === 'Player') {
    const joinedName = joinResult.data?.join?.name as string | undefined
    return (
      <main className="content">
        <h2>{t('inviteJoinTitle')}</h2>
        {joinedName ? (
          <>
            <p>
              {t('inviteJoinedPrefix')} <strong>{joinedName}</strong>.
            </p>
            <Link to="/scoreboard">{t('inviteGoScoreboard')}</Link>
          </>
        ) : (
          <>
            <p>{t('inviteJoinBody')}</p>
            {joinResult.error && (
              <p className="flash-bar">
                {t('errorPrefix')}: {joinResult.error.message}
              </p>
            )}
            <button
              className="primary"
              disabled={joinResult.fetching}
              onClick={() => void runJoin({ code })}
            >
              {t('join')}
            </button>
          </>
        )}
      </main>
    )
  }

  // Unclaimed viewer with a matching account — offer to link (AUTH-13).
  if (viewer.linkCandidate) {
    const candidate = viewer.linkCandidate
    return (
      <main className="content">
        <h2>{t('inviteLinkTitle')}</h2>
        <p>{t('inviteLinkBody')}</p>
        {linkResult.error && (
          <p className="flash-bar">
            {t('errorPrefix')}: {linkResult.error.message}
          </p>
        )}
        <button
          className="primary"
          onClick={async () => {
            const res = await runLink({ personId: candidate.personId })
            if (!res.error) navigate('/profile')
          }}
        >
          {t('inviteLinkConfirm')}
        </button>{' '}
        <button onClick={() => navigate('/')}>{t('inviteLinkCancel')}</button>
      </main>
    )
  }

  // Unclaimed, new — set a display name and claim.
  return (
    <main className="content">
      <h2>{t('inviteClaimTitle')}</h2>
      <p>{t('inviteClaimBody')}</p>
      <NameForm
        submitLabel={t('join')}
        busy={claimResult.fetching}
        flash={
          claimResult.error
            ? `${t('errorPrefix')}: ${claimResult.error.message}`
            : null
        }
        onSubmit={async (nick, fullName) => {
          const res = await runClaim({ code, nick, fullName })
          if (!res.error) navigate('/profile')
        }}
      />
    </main>
  )
}
```

- [ ] **Step 2: Verify types + lint**

Run: `cd web && npx tsc -b --noEmit && npm run lint`
Expected: both exit 0.

- [ ] **Step 3: Commit**

```bash
git add web/src/pages/InviteClaimPage.tsx
git commit -m "feat(web): rewrite InviteClaimPage — Continue to join, shared NameForm, i18n"
```

---

## Task 6: Bigger, labelled settings gear (Part D)

**Files:**
- Modify: `web/src/components/SettingsMenu.tsx`
- Modify: `web/src/index.css`

- [ ] **Step 1: Add the visible label**

In `web/src/components/SettingsMenu.tsx`, replace the gear button's children. Change:

```tsx
      >
        {/* gear glyph */}
        <span aria-hidden="true">⚙</span>
      </button>
```

to:

```tsx
      >
        <span aria-hidden="true">⚙</span>
        <span className="settings-gear-label">{t('settings')}</span>
      </button>
```

- [ ] **Step 2: Make the gear bigger, brighter, inline with its label**

In `web/src/index.css`, replace the `.settings-gear { ... }` rule (the block starting `.settings-gear {` with `font-size: 18px;`) with:

```css
.settings-gear {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 22px;
  line-height: 1;
  padding: 4px 8px;
  background: transparent;
  border: none;
  color: var(--text-on-dark);
  cursor: pointer;
}

.settings-gear-label {
  font-size: 13px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
```

(Leave the existing `.settings-gear:hover, .settings-gear[aria-expanded='true']` rule as-is.)

- [ ] **Step 3: Verify types, lint, build**

Run: `cd web && npx tsc -b --noEmit && npm run lint && npm run build`
Expected: all exit 0.

- [ ] **Step 4: Commit**

```bash
git add web/src/components/SettingsMenu.tsx web/src/index.css
git commit -m "feat(web): bigger, labelled settings gear"
```

---

## Task 7: E2E — update stale assertion + cover the new UI

**Files:**
- Modify: `web/e2e/invite-entry.spec.ts`
- Create: `web/e2e/onboarding.spec.ts`

- [ ] **Step 1: Update the stale invite-entry assertion**

In `web/e2e/invite-entry.spec.ts`, in the first test, replace:

```ts
  await expect(page).toHaveURL(/\/invite\/ABC123XYZ0$/)
  // On the claim page a logged-out viewer is prompted to sign in.
  await expect(page.getByText('Log in to claim this invite.')).toBeVisible()
```

with:

```ts
  await expect(page).toHaveURL(/\/invite\/ABC123XYZ0$/)
  // On the claim page a logged-out viewer is invited to establish identity.
  await expect(
    page.getByRole('button', { name: 'Continue to join' }),
  ).toBeVisible()
```

- [ ] **Step 2: Create the onboarding spec**

Create `web/e2e/onboarding.spec.ts`:

```ts
import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * First-run UX: the invite link shows a clear "Continue to join" entry, the
 * settings gear is labelled, and the Profile form (now the shared NameForm)
 * still renders its labelled fields.
 */

test('the settings trigger shows a visible "Settings" label', async ({ page }) => {
  await page.goto('/')
  await expect(page.locator('.settings-gear')).toContainText('Settings')
})

test('the invite welcome shows "Continue to join" for a logged-out visitor', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/invite/SOUTH7K-AD9XK3P7QT')
  await expect(page.locator('main.content h2')).toBeVisible()
  await expect(
    page.getByRole('button', { name: 'Continue to join' }),
  ).toBeVisible()
  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('the Profile form renders labelled name fields and Save (shared NameForm)', async ({
  page,
}) => {
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/profile')
  const form = page.locator('form.form')
  await expect(form).toBeVisible()
  await expect(form.locator('label', { hasText: 'Nick' })).toBeVisible()
  await expect(form.locator('label', { hasText: 'Full name' })).toBeVisible()
  await expect(form.getByRole('button', { name: 'Save' })).toBeVisible()
})
```

- [ ] **Step 3: Run the affected e2e specs**

Run: `cd web && npm run e2e -- invite-entry onboarding`
Expected: PASS — invite-entry's three tests (with the updated assertion), and onboarding's three tests. (The harness boots the full stack; first run is slow.)

- [ ] **Step 4: Commit**

```bash
git add web/e2e/invite-entry.spec.ts web/e2e/onboarding.spec.ts
git commit -m "test(web): e2e for Continue-to-join, settings label, Profile NameForm"
```

---

## Task 8: Full verification + integrate

**Files:** none (verification + merge)

- [ ] **Step 1: Lint, build, unit**

Run: `cd web && npm run lint && npm run build && npm test`
Expected: all PASS (unit count = prior 115 + 3 new `returnTo` tests = 118).

- [ ] **Step 2: Full e2e suite**

Run: `cd web && npm run e2e`
Expected: all PASS — no cross-spec regressions; the rewritten invite page and shared NameForm hold up with the whole suite running serially.

- [ ] **Step 3: Merge to master locally**

```bash
cd /Users/xczimi/Private/SoccerPool/xpool
git checkout master
git merge --no-ff onboarding-first-run -m "feat(web): onboarding & first-run UX improvements"
```

(If a worktree was used, follow the finishing-a-development-branch flow: verify the merged result, then remove the worktree and delete the branch.)

---

## Self-review notes

- **Spec coverage:** Part A (preserve code) → Tasks 1+2; Part A (Continue to join) → Task 5 + i18n in Task 4; Part B (shared form) → Tasks 3 + 5; Part C (header wording) → Task 4; Part D (gear) → Task 6; testing → Tasks 1, 7, 8. "No API change" honoured — all files under `web/`.
- **Type/name consistency:** `stashReturnTo(path, storage?)` / `takeReturnTo(storage?)` identical across helper, test, `auth0Provider`, `PostLoginRedirect`. `NameForm` props (`initialNick`, `initialFullName`, `submitLabel`, `busy`, `flash`, `onSubmit(nick, fullName)`) identical across definition, `ProfilePage`, `InviteClaimPage`. New i18n keys referenced in Task 5 are all added in Task 4 (so `tsc`'s `StringKey` type resolves).
- **No placeholders:** every code step shows complete code; every run step has an expected result.
- **Mode safety:** `InviteClaimPage` calls `useAuth0()` unconditionally (safe stub in dev) and only invokes `loginWithRedirect` when `auth0Enabled`; the dev path navigates to `/`. This is what lets the e2e (dev mode) assert the rendered "Continue to join" button without a crash.
