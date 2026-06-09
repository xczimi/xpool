# Identity-aware Home / welcome page — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Home page identity-aware — a non-player (logged-out or authenticated-unclaimed) can paste an invite code there and be routed to `/invite/:code`, while a Player sees quick-action links.

**Architecture:** Extract the paste-and-route widget out of `NeedsInvite` into a small, auth-free `InviteCodeEntry` component. `NeedsInvite` (the dead-end / `/invite` page) renders it wrapped in its explainer + Log Out. `HomePage` queries `me` (paused without a session) and branches on identity: non-player → `<InviteCodeEntry/>`; Player → action links; loading → neutral welcome only.

**Tech Stack:** React 19 + TypeScript, urql (GraphQL), react-router, vitest (pure-logic units), Playwright (UI/e2e). UI behaviour is verified by e2e — the project has no React component test harness; the only pure logic here (`extractCode`) is already unit-covered in `web/src/components/inviteCode.test.ts`.

**Spec:** `docs/superpowers/specs/2026-06-09-home-identity-aware-welcome-design.md`

---

## File Structure

- **Create:** `web/src/components/InviteCodeEntry.tsx` — the paste input + Open button + bad-code warning; `extractCode()` → `navigate('/invite/:code')`. No auth coupling.
- **Modify:** `web/src/components/NeedsInvite.tsx` — render `<InviteCodeEntry/>` instead of its inline widget; keep explainer + Log Out.
- **Modify:** `web/src/index.css` — rename `.needs-invite-link` rules to `.invite-code-entry` (the widget now owns the class; the dead-end's `.needs-invite` container rules stay).
- **Rewrite:** `web/src/pages/HomePage.tsx` — identity branches.
- **Create:** `web/e2e/home-welcome.spec.ts` — the feature's e2e.

No new i18n strings: the non-player block reuses `InviteCodeEntry`'s own `inviteOnly*` labels and the existing `homeIntro` (already says "invite-only"); the Player branch reuses `navMyTips` / `navToday` / `navScoreboard` / `navPools`.

All commands run from `web/` unless noted. `npm run e2e` boots its own isolated stack (~30 s) per invocation.

---

### Task 1: e2e first — the Home welcome behaviour (RED)

**Files:**
- Create: `web/e2e/home-welcome.spec.ts`

- [ ] **Step 1: Write the failing e2e spec**

Create `web/e2e/home-welcome.spec.ts`:

```ts
import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * The Home page is identity-aware (design:
 * docs/superpowers/specs/2026-06-09-home-identity-aware-welcome-design.md).
 * A non-player can paste an invite code on Home and be routed to the claim
 * page; a Player sees quick-action links and no invite entry. Locators are
 * scoped to `.page` (the HomePage section) so they never match the NavBar.
 */

test('a logged-out visitor enters an invite code on Home and is routed to the claim page', async ({
  page,
}) => {
  const net = watchNetwork(page)
  // A fresh test context is logged out.
  await page.goto('/')

  const home = page.locator('.page')
  const box = home.getByPlaceholder('Paste your invite link or code')
  await expect(box).toBeVisible()
  await box.fill('ABC123XYZ0')
  await home.getByRole('button', { name: 'Open' }).click()

  await expect(page).toHaveURL(/\/invite\/ABC123XYZ0$/)
  await expect(page.getByText('Log in to claim this invite.')).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('a logged-in player sees action links on Home and no invite entry', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/')

  const home = page.locator('.page')
  await expect(home.getByRole('link', { name: 'My Tips' })).toBeVisible()
  await expect(home.getByRole('link', { name: 'Pools' })).toBeVisible()
  await expect(
    home.getByPlaceholder('Paste your invite link or code'),
  ).toHaveCount(0)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Run the spec to verify it fails**

Run: `npm run e2e -- home-welcome.spec.ts`
Expected: BOTH tests FAIL. Visitor test: `getByPlaceholder('Paste your invite link or code')` not found on Home (no entry yet). Player test: `.page` `My Tips` link not visible (today's Home links are Today/Scoreboard/Schedule/Rules, not My Tips/Pools).

- [ ] **Step 3: Commit the failing test**

```bash
git add web/e2e/home-welcome.spec.ts
git commit -m "test(web): e2e for identity-aware Home welcome (RED)"
```

---

### Task 2: Create the `InviteCodeEntry` component + move its CSS

**Files:**
- Create: `web/src/components/InviteCodeEntry.tsx`
- Modify: `web/src/index.css:870-893`

- [ ] **Step 1: Create `web/src/components/InviteCodeEntry.tsx`**

```tsx
import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useI18n } from '../i18n/useI18n'
import { extractCode } from './inviteCode'

/**
 * The recipient-side invite widget in isolation: paste a link or bare code,
 * `extractCode` normalises it, and we route to the public claim page
 * (`/invite/:code`). No auth coupling, no logout — usable anywhere (the Home
 * welcome, the `NeedsInvite` dead-end). See
 * docs/superpowers/specs/2026-06-09-home-identity-aware-welcome-design.md.
 */
export function InviteCodeEntry() {
  const { t } = useI18n()
  const navigate = useNavigate()
  const [entry, setEntry] = useState('')
  const [bad, setBad] = useState(false)

  const open = () => {
    const code = extractCode(entry)
    if (!code) {
      setBad(true)
      return
    }
    navigate(`/invite/${code}`)
  }

  return (
    <div className="invite-code-entry">
      <label>
        {t('inviteOnlyHaveLink')}
        <input
          type="text"
          value={entry}
          placeholder={t('inviteOnlyPastePlaceholder')}
          onChange={(e) => {
            setEntry(e.target.value)
            if (bad) setBad(false)
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter') open()
          }}
        />
      </label>
      <button type="button" onClick={open}>
        {t('inviteOnlyOpen')}
      </button>
      {bad && <p className="auth-warn">{t('inviteOnlyBadLink')}</p>}
    </div>
  )
}
```

- [ ] **Step 2: Rename the widget CSS in `web/src/index.css`**

Replace the four `.needs-invite-link…` rules (lines 870-893) with the same rules under `.invite-code-entry`. Leave lines 866-869 (`.needs-invite` container) unchanged.

Old:
```css
.needs-invite-link {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: 8px;
  margin: 12px 0;
}
.needs-invite-link label {
  flex: 1 1 18rem;
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  color: var(--text-dim);
}
.needs-invite-link input {
  width: 100%;
  font-family: 'Share Tech Mono', monospace;
  font-size: 13px;
  color: var(--amber-bright);
  background: var(--bg-input);
  letter-spacing: 0.5px;
}
.needs-invite-link .auth-warn { flex-basis: 100%; margin-left: 0; }
```

New:
```css
.invite-code-entry {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: 8px;
  margin: 12px 0;
}
.invite-code-entry label {
  flex: 1 1 18rem;
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  color: var(--text-dim);
}
.invite-code-entry input {
  width: 100%;
  font-family: 'Share Tech Mono', monospace;
  font-size: 13px;
  color: var(--amber-bright);
  background: var(--bg-input);
  letter-spacing: 0.5px;
}
.invite-code-entry .auth-warn { flex-basis: 100%; margin-left: 0; }
```

- [ ] **Step 3: Type-check + lint**

Run: `npm run build && npm run lint`
Expected: both succeed. (`InviteCodeEntry` is unused so far — that's fine; tsc flags unused *locals*, not unused exports.)

- [ ] **Step 4: Commit**

```bash
git add web/src/components/InviteCodeEntry.tsx web/src/index.css
git commit -m "feat(web): extract InviteCodeEntry widget + move its CSS"
```

---

### Task 3: Refactor `NeedsInvite` to use `InviteCodeEntry`

**Files:**
- Modify: `web/src/components/NeedsInvite.tsx`

- [ ] **Step 1: Replace the whole file contents**

`web/src/components/NeedsInvite.tsx` becomes:

```tsx
import { useI18n } from '../i18n/useI18n'
import { useAuth } from '../auth/useAuth'
import { InviteCodeEntry } from './InviteCodeEntry'

/**
 * The dead-end for an authenticated viewer who is not yet a Player and has no
 * link candidate (invite-only-hardening). Shown in the content area in place of
 * a player-only page, and also rendered at the public `/invite` route. Public
 * pages stay reachable — see `accessFor` in `auth/routeAccess.ts`.
 *
 * The way out is `InviteCodeEntry`: it extracts the code from a pasted link or
 * bare code and routes to the public claim page (`/invite/:code`).
 */
export function NeedsInvite() {
  const { t } = useI18n()
  const { logout } = useAuth()

  return (
    <div className="status needs-invite">
      <h2>{t('inviteOnlyTitle')}</h2>
      <p>{t('inviteOnlyBody')}</p>

      <InviteCodeEntry />

      <button type="button" onClick={logout}>
        {t('logOut')}
      </button>
    </div>
  )
}
```

- [ ] **Step 2: Type-check + lint**

Run: `npm run build && npm run lint`
Expected: both succeed. (The removed `useState`/`useNavigate`/`extractCode` imports are gone; no unused-import errors.)

- [ ] **Step 3: Run the regression e2e for the dead-end + `/invite` page**

Run: `npm run e2e -- auth.spec.ts invite-entry.spec.ts`
Expected: PASS. The DOM contract `NeedsInvite` exposes (the `Paste your invite link or code` placeholder, the `Open` button, the `You need an invite` heading) is unchanged, so the dead-end test and the public-`/invite` tests still pass.

- [ ] **Step 4: Commit**

```bash
git add web/src/components/NeedsInvite.tsx
git commit -m "refactor(web): NeedsInvite renders the shared InviteCodeEntry"
```

---

### Task 4: Make `HomePage` identity-aware

**Files:**
- Modify: `web/src/pages/HomePage.tsx`

- [ ] **Step 1: Replace the whole file contents**

`web/src/pages/HomePage.tsx` becomes:

```tsx
import { Link } from 'react-router-dom'
import { useQuery } from 'urql'
import { useI18n } from '../i18n/useI18n'
import { useAuth } from '../auth/useAuth'
import { ME_QUERY } from '../graphql/queries'
import type { Me } from '../graphql/types'
import { InviteCodeEntry } from '../components/InviteCodeEntry'

/**
 * Identity-aware welcome (design:
 * docs/superpowers/specs/2026-06-09-home-identity-aware-welcome-design.md).
 * A non-player (logged-out or authenticated-unclaimed) gets the invite-code
 * entry — the front door to a pool. A Player gets quick-action links. While a
 * session's `me` is still resolving we show only the neutral welcome, to avoid
 * a flash of the wrong branch. `me` is paused without a session (mirrors
 * `Layout`/`PoolsPage`), so a logged-out viewer fires no auth query.
 */
export function HomePage() {
  const { t } = useI18n()
  const { label } = useAuth()
  const [meResult] = useQuery<{ me: Me }>({ query: ME_QUERY, pause: !label })

  const isPlayer = meResult.data?.me?.__typename === 'Player'
  const loading = Boolean(label) && meResult.fetching && !meResult.data

  return (
    <section className="page">
      <h2>{t('homeWelcome')}</h2>
      <p>{t('homeIntro')}</p>

      {!loading &&
        (isPlayer ? (
          <div className="home-links">
            <Link to="/mytips">{t('navMyTips')}</Link>
            <Link to="/today">{t('navToday')}</Link>
            <Link to="/scoreboard">{t('navScoreboard')}</Link>
            <Link to="/pools">{t('navPools')}</Link>
          </div>
        ) : (
          <InviteCodeEntry />
        ))}
    </section>
  )
}
```

- [ ] **Step 2: Type-check + lint**

Run: `npm run build && npm run lint`
Expected: both succeed.

- [ ] **Step 3: Commit**

```bash
git add web/src/pages/HomePage.tsx
git commit -m "feat(web): identity-aware Home — invite entry for non-players, links for players"
```

---

### Task 5: Verify the feature e2e is GREEN (and nothing regressed)

**Files:** none (verification gate)

- [ ] **Step 1: Run the feature spec**

Run: `npm run e2e -- home-welcome.spec.ts`
Expected: BOTH tests PASS (the RED from Task 1 is now green).

- [ ] **Step 2: Run the full invite/auth/pools regression set**

Run: `npm run e2e -- home-welcome.spec.ts invite-entry.spec.ts auth.spec.ts pools.spec.ts visitor-smoke.spec.ts`
Expected: all PASS. Confirms the `NeedsInvite` refactor and Home rewrite broke nothing in the invite/claim/dead-end/pools flows.

- [ ] **Step 3: Final unit + lint sweep**

Run: `npm test && npm run lint`
Expected: vitest passes (incl. the existing `extractCode` suite); eslint clean.

- [ ] **Step 4: No commit needed** if Steps 1-3 produced no file changes. If any fix was required, commit it with a `fix(web): …` message describing the fix.

---

## Self-Review

**Spec coverage:**
- "non-player can enter a code on Home → /invite/:code" → Task 4 (non-player branch) + Task 1 visitor test. ✓
- Visitor and Unclaimed share one non-player block → Task 4 (`isPlayer` is the only branch; both non-Player states fall through to `<InviteCodeEntry/>`). ✓
- Player branch = action links (My Tips, Today, Scoreboard, Pools) → Task 4 + Task 1 player test. ✓
- Loading = neutral welcome only → Task 4 (`loading` guard). ✓
- `InviteCodeEntry` extracted, no auth/logout → Task 2. ✓
- `NeedsInvite` reuses it, contract unchanged → Task 3 + its regression run. ✓
- Reuse `inviteOnly*` + `homeIntro`, no new strings → Tasks 2 & 4 (no `strings.ts` edit). ✓
- Testing: e2e visitor + player; dead-end regression; `extractCode` already unit-covered → Tasks 1, 3, 5. ✓
- Deferred (tournament-"day", D6 logout guard) → not in plan, by design. ✓

**Placeholder scan:** none — every code/CSS step shows full content; every run step shows command + expected result.

**Type consistency:** `Me` imported from `../graphql/types` (same as `PoolsPage`); `me.__typename === 'Player'` matches `ME_QUERY`'s union; `InviteCodeEntry` exported from `web/src/components/InviteCodeEntry.tsx` and imported by both `NeedsInvite` (`./InviteCodeEntry`) and `HomePage` (`../components/InviteCodeEntry`); CSS class `invite-code-entry` matches the component's `className`.
