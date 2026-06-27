# Cluster: cross-cutting-ux — page one-liner intros Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give every page an always-visible, i18n'd one-line intro under its heading, via one shared `PageHeading` component.

**Architecture:** There is no existing shared page-header component — every page renders `<section className="page"><h2>{t('…Title')}</h2>…</section>` inline. We introduce one small presentational component, `PageHeading`, that owns the title + subtitle markup and the title divider, and thread it through every `.page` page. Each page passes a `titleKey` and an `introKey`; the intro sentence is a plain i18n'd string keyed beside the existing per-page title key in `strings.ts` (EN + HU). The subtitle is always shown — not dismissible, not first-visit-only — and does **not** double as `<title>`/meta this round.

**Tech Stack:** React + Vite + TypeScript SPA (`web/`), urql GraphQL client, custom i18n (`web/src/i18n/strings.ts` + `useI18n`), Playwright e2e (`web/e2e/`), CSS design system in `web/src/index.css` (LED/scoreboard amber theme, `Share Tech Mono`).

## Global Constraints

- **Display name is "xPool"** (repo/crate identifiers stay lowercase `xpool`). Copy must say xPool, never "XPool"/"Xpool".
- **i18n is first-class — every new string needs BOTH `en` and `hu`** in `web/src/i18n/strings.ts`. `StringKey = keyof typeof en`; the `hu` block must mirror `en` key-for-key or `catalogues: Record<Locale, Record<StringKey, string>>` fails to type-check (`npm run build`).
- **`web/src/i18n/strings.ts` is an append-only SHARED SEAM.** Every cluster this round appends keys to it. Add new keys beside the matching per-page `…Title` key; do not reorder/rename existing keys. Merge conflicts here are reconciled at integration — keep additions localised and additive.
- **One shared pattern, not bespoke per page.** All subtitle markup lives in `PageHeading`; pages never hand-roll a `<p className="page-intro">`.
- **Immutability; many small focused files.** `PageHeading` is its own file (~25 lines).
- **Server-authoritative clock — no `Date.now()`** in any code this plan touches (presentational only; this is a non-issue but holds).
- **HU register matches the existing casual catalogue** (`Tippverseny`, `Ligák`, `Adatok`, `tuti`/`tutiban`, `Szevasztok!`) — keep intros short and friendly, not formal.

## File Structure

- **Create** `web/src/components/PageHeading.tsx` — the shared title+subtitle component (one responsibility: render a page heading with an optional always-visible intro line).
- **Create** `web/e2e/page-intros.spec.ts` — e2e proving public pages render their intro and that it localises EN↔HU.
- **Modify** `web/src/i18n/strings.ts` — append 10 `*Intro` keys to the `en` block (beside each title) and 10 mirrors to the `hu` block. (`homeIntro` already exists and is reused — no new home key.)
- **Modify** `web/src/index.css` — add `.page-heading` / `.page-intro` rules (placed AFTER `.page h2`/`.page h3`/`.page h4` so the heading-scoped overrides win on equal specificity).
- **Modify** 11 page files to route their heading through `PageHeading`:
  `HomePage.tsx`, `MyTipsPage.tsx`, `SchedulePage.tsx`, `TodayPage.tsx`,
  `ScoreboardPage.tsx`, `AllTipsPage.tsx`, `PerfectPage.tsx`, `PoolsPage.tsx`,
  `RulesPage.tsx`, `ProfilePage.tsx`, `AdminPage.tsx`.

### Pages deliberately NOT changed (documented decisions, not gaps)

The PRD lists `InvitePage` and `InviteClaimPage`. In the codebase:

- **`InvitePage` = the `/invite` route = `components/NeedsInvite.tsx`.** It renders in a distinct `.status needs-invite` layout (not `.page`) and **already carries an explanatory subtitle** under its heading: `<p>{t('inviteOnlyBody')}</p>`. It is already "covered" by the pattern's intent. Forcing `.page-heading` into the `.status` block would fight its bespoke styling. **Leave as-is.**
- **`InviteClaimPage.tsx`** renders one of four states (welcome / join / link / claim) in a `.content` layout (not `.page`), and **each branch already has its own body paragraph** under its `<h2>` (`inviteWelcomeBody`, `inviteJoinBody`, `inviteLinkBody`, `inviteClaimBody`). Already covered. **Leave as-is.**
- **Player-analytics pages added in Wave 1 (H2H + points-timeline), and `MatchPage`/`PlayerPage`/`PrivacyPage`,** are out of this cluster's covered-pages list. If the H2H / timeline pages exist at integration time, add a `PageHeading` to them as a **follow-up** (same component + two new `*Intro` keys each) — do not hard-code them here.

---

### Task 1: Failing e2e — public pages render a localised intro

Write the e2e first so the rest of the work has a red bar to turn green. `/rules` and `/scoreboard` are both public (the scoreboard page is public; only its pool selector needs auth), so a logged-out visitor proves the pattern with no dev login. This mirrors the existing `web/e2e/rules-content.spec.ts` (same helpers, same language-switch flow via the settings menu).

**Files:**
- Create: `web/e2e/page-intros.spec.ts`

**Interfaces:**
- Consumes (existing helpers, already used by `rules-content.spec.ts`): `expectNoErrorView(page)`, `openSettings(page)`, `watchNetwork(page) → { assertNoGraphqlErrors(): Promise<void>, assertNoPageErrors(): void }` from `web/e2e/helpers.ts`.
- Produces: the spec that Task 5 runs green. Asserts the exact intro strings added in Task 3 — keep these byte-identical (em dash `—`, curly-free ASCII apostrophe `'` as written below).

- [ ] **Step 1: Write the failing test**

Create `web/e2e/page-intros.spec.ts`:

```ts
import { test, expect } from '@playwright/test'
import { expectNoErrorView, openSettings, watchNetwork } from './helpers'

/**
 * page-one-liner-intros: every page carries an always-visible one-line intro
 * under its heading, i18n'd EN + HU. `/rules` and `/scoreboard` are both public,
 * so a logged-out visitor proves the shared pattern without dev auth. The
 * language switch (settings menu → Magyar) re-renders the same intros localised.
 */
test('public pages render their one-line intro, localised EN/HU', async ({
  page,
}) => {
  const net = watchNetwork(page)

  // English baseline — the intro lines under each page heading.
  await page.goto('/rules')
  await expect(
    page.getByText('How predictions are scored, point by point.'),
  ).toBeVisible()

  await page.goto('/scoreboard')
  await expect(
    page.getByText("Who's winning — total points across the pool."),
  ).toBeVisible()

  // Switch UI language; the same intros render localised (not English-only).
  await openSettings(page)
  await page.getByRole('radio', { name: 'Magyar' }).click()
  await expect(
    page.getByText('Ki vezet — összpontszámok az egész tutiban.'),
  ).toBeVisible()

  await page.goto('/rules')
  await expect(
    page.getByText('Hogyan pontozzuk a tippeket, pontról pontra.'),
  ).toBeVisible()
  await expect(
    page.getByText('How predictions are scored, point by point.'),
  ).toHaveCount(0)

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd web && npm run e2e -- page-intros`
Expected: FAIL — the intro strings don't exist yet, so `getByText(...)` times out (the Rules/Scoreboard pages render their `<h2>` but no `.page-intro`). This boots the e2e stack itself (own ports `:3001`/`:5174`/`:8001`); it coexists with any running `bin/local-dev`.

- [ ] **Step 3: Commit the failing test**

```bash
cd /Users/xczimi/Private/SoccerPool/xpool
git add web/e2e/page-intros.spec.ts
git commit -m "test(web): e2e for always-visible page one-liner intros (red)"
```

---

### Task 2: `PageHeading` shared component + CSS

The one place that owns title + subtitle markup and the heading divider.

**Files:**
- Create: `web/src/components/PageHeading.tsx`
- Modify: `web/src/index.css` (append `.page-heading` / `.page-intro` rules after `.page h4` — current line 346)

**Interfaces:**
- Consumes: `useI18n()` from `web/src/i18n/useI18n` (returns `{ t, locale, … }`; `t(key: StringKey) → string`), and `StringKey` (`= keyof typeof en`) from `web/src/i18n/strings`.
- Produces: `PageHeading({ titleKey, introKey }: { titleKey: StringKey; introKey?: StringKey })` — renders `<header className="page-heading"><h2>{t(titleKey)}</h2>{introKey && <p className="page-intro">{t(introKey)}</p>}</header>`. Tasks 3–4 rely on exactly these prop names and that both are `StringKey`s.

- [ ] **Step 1: Create the component**

Create `web/src/components/PageHeading.tsx`:

```tsx
import { useI18n } from '../i18n/useI18n'
import type { StringKey } from '../i18n/strings'

/**
 * Shared page heading: the page title plus an always-visible one-line intro
 * (page-one-liner-intros). One place owns the title/subtitle markup and the
 * divider, so every `.page` reads consistently. The intro is a plain i18n'd
 * sentence keyed beside the title in `strings.ts` (EN + HU). It is not
 * dismissible and does not double as `<title>`/meta.
 */
export function PageHeading({
  titleKey,
  introKey,
}: {
  titleKey: StringKey
  introKey?: StringKey
}) {
  const { t } = useI18n()
  return (
    <header className="page-heading">
      <h2>{t(titleKey)}</h2>
      {introKey && <p className="page-intro">{t(introKey)}</p>}
    </header>
  )
}
```

- [ ] **Step 2: Add the CSS**

In `web/src/index.css`, the existing block ends at line 346:

```css
.page h3 { font-size: 13px; margin-bottom: 10px; text-transform: uppercase; }
.page h4 { font-size: 11px; margin-bottom: 8px; text-transform: uppercase; }
```

Immediately AFTER that `.page h4` line, insert:

```css

/* page-one-liner-intros: shared title + always-visible subtitle. The wrapper
   owns the divider so the intro sits tight under the title, above the rule.
   Placed after `.page h2` so these heading-scoped overrides win on equal
   specificity (source order). */
.page-heading {
  margin-bottom: 16px;
  border-bottom: 1px solid var(--amber-dim);
  padding-bottom: 10px;
}
.page-heading h2 {
  margin-bottom: 0;
  border-bottom: none;
  padding-bottom: 0;
}
.page-intro {
  margin: 6px 0 0;
  font-family: 'Share Tech Mono', monospace;
  font-size: 12px;
  color: var(--text-dim);
  letter-spacing: 0.5px;
  text-transform: none;
}
```

Note: `--amber-dim` and `--text-dim` are existing theme variables (already used by `.page h2` and `.hint` respectively), so the subtitle matches the LED/scoreboard design system. `.page-heading h2` removes the title's own bottom border (the wrapper now carries it).

- [ ] **Step 3: Verify it compiles**

Run: `cd web && npm run build`
Expected: PASS (`tsc -b && vite build` succeeds; `PageHeading` is not yet imported anywhere, which is fine — TS allows an unused module).

- [ ] **Step 4: Commit**

```bash
cd /Users/xczimi/Private/SoccerPool/xpool
git add web/src/components/PageHeading.tsx web/src/index.css
git commit -m "feat(web): shared PageHeading with always-visible intro slot + CSS"
```

---

### Task 3: i18n intro strings (EN + HU)

Append one `*Intro` key per covered page, beside the matching `*Title`. `homeIntro` already exists (line 118 EN / 472 HU) and is reused by HomePage — do not duplicate it.

**Files:**
- Modify: `web/src/i18n/strings.ts` (the `en` object and the `hu` object)

**Interfaces:**
- Produces these `StringKey`s (consumed by Task 4): `todayIntro`, `scheduleIntro`, `myTipsIntro`, `allTipsIntro`, `scoreboardIntro`, `perfectIntro`, `poolsIntro`, `profileIntro`, `rulesIntro`, `adminIntro`. Plus the pre-existing `homeIntro`.

- [ ] **Step 1: Add the English keys**

In the `en` object, add each `*Intro` immediately below its sibling `*Title`:

- Below `todayTitle: 'Today / Fresh',` (line 136):
  ```ts
  todayIntro: 'Matches kicking off today — and the deadlines to beat.',
  ```
- Below `scheduleTitle: 'Schedule',` (line 142):
  ```ts
  scheduleIntro: 'Every match of the tournament, by group or by date.',
  ```
- Below `myTipsTitle: 'My Tips',` (line 167):
  ```ts
  myTipsIntro: 'Enter and edit your score predictions, group by group.',
  ```
- Below `allTipsTitle: 'All Tips',` (line 194):
  ```ts
  allTipsIntro: "See everyone's predictions once a match kicks off.",
  ```
- Below `scoreboardTitle: 'Scoreboard',` (line 207):
  ```ts
  scoreboardIntro: "Who's winning — total points across the pool.",
  ```
- Below `perfectTitle: 'Perfect Predictions',` (line 214):
  ```ts
  perfectIntro: 'Your spot-on predictions — exact scores you nailed.',
  ```
- Below `poolsTitle: 'Pools',` (line 229):
  ```ts
  poolsIntro: 'Create or manage your pools and invite friends.',
  ```
- Below `profileTitle: 'Profile',` (line 274):
  ```ts
  profileIntro: 'Your account details and display name.',
  ```
- Below `rulesTitle: 'Rules & Scoring',` (line 291):
  ```ts
  rulesIntro: 'How predictions are scored, point by point.',
  ```
- Below `adminTitle: 'Admin',` (line 341):
  ```ts
  adminIntro: 'Enter official results and manage the tournament.',
  ```

- [ ] **Step 2: Add the matching Hungarian keys**

In the `hu` object, mirror each one below its sibling `*Title` (casual register; `tuti`/`Ligák` match the catalogue):

- Below `todayTitle: 'Ma / Friss',` (line 489):
  ```ts
  todayIntro: 'A ma kezdődő meccsek — és a határidők, amiket be kell tartani.',
  ```
- Below `scheduleTitle: 'Menetrend',` (line 494):
  ```ts
  scheduleIntro: 'A torna összes meccse, csoport vagy dátum szerint.',
  ```
- Below `myTipsTitle: 'Tippjeim',` (line 517):
  ```ts
  myTipsIntro: 'Írd be és módosítsd a tippjeidet, csoportról csoportra.',
  ```
- Below `allTipsTitle: 'Összes tipp',` (line 543):
  ```ts
  allTipsIntro: 'Nézd meg mindenki tippjét, amint egy meccs elkezdődött.',
  ```
- Below `scoreboardTitle: 'Tippverseny',` (line 554):
  ```ts
  scoreboardIntro: 'Ki vezet — összpontszámok az egész tutiban.',
  ```
- Below `perfectTitle: 'Telitalálatok',` (line 560):
  ```ts
  perfectIntro: 'A telitalálataid — pontos eredmények, amiket eltaláltál.',
  ```
- Below `poolsTitle: 'Ligák',` (line 574):
  ```ts
  poolsIntro: 'Hozd létre vagy kezeld a ligáidat, és hívj meg barátokat.',
  ```
- Below `profileTitle: 'Adatok',` (line 618):
  ```ts
  profileIntro: 'A fiókod adatai és a megjelenített neved.',
  ```
- Below `rulesTitle: 'Szabályok és pontozás',` (line 633):
  ```ts
  rulesIntro: 'Hogyan pontozzuk a tippeket, pontról pontra.',
  ```
- Below `adminTitle: 'Admin',` (line 682):
  ```ts
  adminIntro: 'Hivatalos eredmények rögzítése és a torna kezelése.',
  ```

- [ ] **Step 3: Verify EN/HU parity type-checks**

Run: `cd web && npm run build`
Expected: PASS. If a key was added to `en` but not `hu` (or vice-versa), `catalogues: Record<Locale, Record<StringKey, string>>` (end of `strings.ts`) fails: TS error "Property 'xIntro' is missing in type … hu". Fix by adding the missing mirror.

- [ ] **Step 4: Commit**

```bash
cd /Users/xczimi/Private/SoccerPool/xpool
git add web/src/i18n/strings.ts
git commit -m "feat(web): EN+HU one-liner intro strings beside each page title"
```

---

### Task 4: Route every covered page heading through `PageHeading`

Replace the inline `<h2>{t('…Title')}</h2>` (and, for Home, the extra `<p>{t('homeIntro')}</p>`) with `<PageHeading titleKey="…" introKey="…" />`, adding the import to each file. The `t` call elsewhere in every page keeps `useI18n`/`t` in use, so no import becomes dead.

**Files (each: add import + replace heading):**
- Modify: `web/src/pages/HomePage.tsx`
- Modify: `web/src/pages/TodayPage.tsx`
- Modify: `web/src/pages/SchedulePage.tsx`
- Modify: `web/src/pages/MyTipsPage.tsx`
- Modify: `web/src/pages/AllTipsPage.tsx`
- Modify: `web/src/pages/ScoreboardPage.tsx`
- Modify: `web/src/pages/PerfectPage.tsx`
- Modify: `web/src/pages/PoolsPage.tsx`
- Modify: `web/src/pages/ProfilePage.tsx`
- Modify: `web/src/pages/RulesPage.tsx`
- Modify: `web/src/pages/AdminPage.tsx`

**Interfaces:**
- Consumes: `PageHeading` from `'../components/PageHeading'` (Task 2); the `*Intro` keys (Task 3).

- [ ] **Step 1: HomePage — reuse the existing `homeIntro`**

Add the import after the existing component imports (e.g. after the `InviteCodeEntry` import line):
```tsx
import { PageHeading } from '../components/PageHeading'
```
Replace:
```tsx
      <h2>{t('homeWelcome')}</h2>
      <p>{t('homeIntro')}</p>
```
with:
```tsx
      <PageHeading titleKey="homeWelcome" introKey="homeIntro" />
```

- [ ] **Step 2: TodayPage**

Add `import { PageHeading } from '../components/PageHeading'` with the other component imports. Replace:
```tsx
      <h2>{t('todayTitle')}</h2>
```
with:
```tsx
      <PageHeading titleKey="todayTitle" introKey="todayIntro" />
```

- [ ] **Step 3: SchedulePage**

Add the import. Replace:
```tsx
      <h2>{t('scheduleTitle')}</h2>
```
with:
```tsx
      <PageHeading titleKey="scheduleTitle" introKey="scheduleIntro" />
```

- [ ] **Step 4: MyTipsPage**

Add the import (alongside `import { GroupTipForm } from './mytips/GroupTipForm'` etc.). Replace:
```tsx
      <h2>{t('myTipsTitle')}</h2>
```
with:
```tsx
      <PageHeading titleKey="myTipsTitle" introKey="myTipsIntro" />
```

- [ ] **Step 5: AllTipsPage**

Add the import. Replace:
```tsx
      <h2>{t('allTipsTitle')}</h2>
```
with:
```tsx
      <PageHeading titleKey="allTipsTitle" introKey="allTipsIntro" />
```

- [ ] **Step 6: ScoreboardPage**

Add `import { PageHeading } from '../components/PageHeading'` (next to `import { ErrorView, Loading } from '../components/StatusViews'`). Replace:
```tsx
      <h2>{t('scoreboardTitle')}</h2>
```
with:
```tsx
      <PageHeading titleKey="scoreboardTitle" introKey="scoreboardIntro" />
```

- [ ] **Step 7: PerfectPage**

Add the import. Replace:
```tsx
      <h2>{t('perfectTitle')}</h2>
```
with:
```tsx
      <PageHeading titleKey="perfectTitle" introKey="perfectIntro" />
```
(The `aria-label={t('perfectTitle')}` on the seg-toggle below stays unchanged.)

- [ ] **Step 8: PoolsPage**

Add the import. Replace:
```tsx
      <h2>{t('poolsTitle')}</h2>
```
with:
```tsx
      <PageHeading titleKey="poolsTitle" introKey="poolsIntro" />
```

- [ ] **Step 9: ProfilePage**

Add the import. Replace:
```tsx
      <h2>{t('profileTitle')}</h2>
```
with:
```tsx
      <PageHeading titleKey="profileTitle" introKey="profileIntro" />
```

- [ ] **Step 10: RulesPage**

Add the import. Replace:
```tsx
      <h2>{t('rulesTitle')}</h2>
```
with:
```tsx
      <PageHeading titleKey="rulesTitle" introKey="rulesIntro" />
```
(The `<h3>` section sub-headings below stay unchanged.)

- [ ] **Step 11: AdminPage**

Add `import { PageHeading } from '../components/PageHeading'` with the other component imports. Replace:
```tsx
      <h2>{t('adminTitle')}</h2>
```
with:
```tsx
      <PageHeading titleKey="adminTitle" introKey="adminIntro" />
```

- [ ] **Step 12: Verify build + lint**

Run: `cd web && npm run build && npm run lint`
Expected: PASS. If lint flags `'t' is defined but never used` on any page, that page used `t` only for its title — re-check; all eleven also call `t` elsewhere, so this should not happen. If `PageHeading` import path is wrong, `tsc` errors "Cannot find module" — fix the relative path (`../components/PageHeading`).

- [ ] **Step 13: Commit**

```bash
cd /Users/xczimi/Private/SoccerPool/xpool
git add web/src/pages/
git commit -m "feat(web): route every page heading through PageHeading with intro"
```

---

### Task 5: Verification + code-review checkpoint

Prove the cluster meets the per-cluster quality bar and hand off for review.

**Files:** none (verification only).

- [ ] **Step 1: e2e green**

Run: `cd web && npm run e2e -- page-intros`
Expected: PASS — the spec from Task 1 now finds the EN intros, switches to Magyar, and finds the HU intros (and the EN string is gone). The stack runs on its own ports and coexists with `bin/local-dev`.

- [ ] **Step 2: Full web gates**

Run: `cd web && npm run build && npm run lint`
Expected: both PASS.

- [ ] **Step 3: Workspace stays green**

Run: `cd /Users/xczimi/Private/SoccerPool/xpool && cargo build --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings`
Expected: PASS (this cluster is web-only; Rust must remain unaffected). DynamoDB integration tests stay skipped without `DYNAMO_TEST=1` — that's green.

- [ ] **Step 4: Visual check (design-system consistency)**

Per CLAUDE.md "verify frontend visually" — green e2e/build/lint is not enough. With a dev stack up (`bin/local-dev`, or `docker compose up -d` + import + seed + `cargo run -p api` + `cd web && npm run dev`), open `/rules`, `/scoreboard`, `/mytips`, `/today`, `/pools` and confirm: the intro sits tight under the title, above the amber divider; it renders in dim `Share Tech Mono` (not uppercased, smaller than the title); the layout doesn't shift the page body. Toggle EN↔HU in the settings menu and confirm each intro localises. Confirm the title says **xPool** branding is unaffected.

- [ ] **Step 5: Request code review**

REQUIRED SUB-SKILL: Use superpowers:requesting-code-review.
Scope of the review: the new `PageHeading` component and its single CSS block; the 20 appended i18n keys (EN/HU parity + casual HU register + xPool branding); the 11 page wirings (no dead `t`/imports, heading divider still reads correctly); the new e2e. Flag for the reviewer that `web/src/i18n/strings.ts` is an **append-only shared seam** other clusters also touch — additions must reconcile cleanly at integration. Note the two deliberate non-changes (`NeedsInvite`/`InvitePage` and `InviteClaimPage` already carry subtitle body copy in their non-`.page` layouts) and the follow-up to add `PageHeading` to the Wave-1 H2H/timeline pages if they exist.

---

## Self-Review

**Spec coverage (PRD resolved decisions):**
- Always-visible subtitle slot under each heading → `PageHeading` always renders `.page-intro` when `introKey` is passed; all 11 `.page` pages pass it (Task 4). ✓
- Not dismissible / not first-visit-only → no conditional/state; pure render. ✓
- Does NOT double as `<title>`/meta → component renders only `<h2>` + `<p>`; no document-title side effects. ✓
- One shared pattern/component → single `PageHeading` (Task 2); pages never hand-roll the markup. ✓
- One i18n'd sentence per page (EN+HU) keyed beside the title → Task 3, 10 new pairs + reused `homeIntro`. ✓
- Covered pages: Home, MyTips, Schedule, Today, Scoreboard, AllTips, Perfect, Pools, Rules, Profile, Admin all wired; Invite/InviteClaim documented as already-covered (existing body copy in distinct layouts); H2H/timeline noted as follow-up. ✓
- Per-cluster quality bar: web build+lint (Tasks 2/4/5), workspace cargo (Task 5), e2e proving intros + EN/HU switch (Tasks 1/5), visual check note (Task 5), CSS in design system (Task 2), e2e on public pages (no auth needed; dev-stub convention noted), no `Date.now()`. ✓

**Placeholder scan:** No TBD/TODO; every code step shows complete code; every command has expected output. ✓

**Type consistency:** `PageHeading({ titleKey, introKey })` with both `StringKey` — used identically across all of Task 4. All ten `*Intro` keys defined in Task 3 are exactly those referenced in Task 4; `homeIntro` is pre-existing and reused. e2e asserts the exact strings added in Task 3. ✓
