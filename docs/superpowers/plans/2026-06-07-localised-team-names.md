# Localised Team Names Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render team names in the user's UI language (Hungarian country names when the locale is `hu`), falling back to the English JSON name.

**Architecture:** A web-only i18n change. A new `teamNames` catalogue keyed by `shortCode` plus a `teamDisplayName(team, locale)` resolver. Localisation is applied once at the `teamIndex(teams, locale)` boundary, so every downstream consumer (team labels, flag alt text, slot labels, admin sort) is localised for free. No domain / API / storage / tournament-JSON changes.

**Tech Stack:** React + TypeScript + Vite, urql GraphQL client, Vitest (unit), Playwright (e2e). i18n via `web/src/i18n/`.

**Spec:** `docs/superpowers/specs/2026-06-07-localised-team-names-design.md`

**Working directory:** worktree `.claude/worktrees/localised-team-names` (branch `localised-team-names`). All paths below are relative to `web/` unless noted. Run all commands from `web/`.

**Conventions:**
- Use `npm run test -- <path>` (or `node_modules/.bin/vitest run <path>`) — never `npx`.
- `Locale` is declared in `web/src/i18n/strings.ts` (`type Locale = 'en' | 'hu'`).
- Immutability: never mutate; return new objects.

---

## File Structure

- **Create** `web/src/i18n/teamNames.ts` — the localised-name catalogue + `teamDisplayName` resolver. One responsibility: map `(shortCode, locale)` → display name.
- **Create** `web/src/i18n/teamNames.test.ts` — unit tests for the resolver.
- **Modify** `web/src/lib/format.ts` — `teamIndex` gains a `locale` argument and localises each team's `name`.
- **Modify** `web/src/lib/format.test.ts` — add `teamIndex` localisation tests.
- **Modify** 6 call sites that build a team index, to pass `locale`:
  - `web/src/pages/SchedulePage.tsx` (already destructures `locale`)
  - `web/src/pages/TodayPage.tsx` (already destructures `locale`)
  - `web/src/pages/PerfectPage.tsx` (add `locale`)
  - `web/src/pages/AllTipsPage.tsx` (add `locale`)
  - `web/src/pages/mytips/GroupTipForm.tsx` (add `locale`)
  - `web/src/pages/admin/AdminTeams.tsx` (add `locale`)
- **Create** `web/e2e/team-names-i18n.spec.ts` — Playwright test: HU shows Hungarian names, EN shows English.

---

## Task 1: Localised-name catalogue + resolver

**Files:**
- Create: `web/src/i18n/teamNames.ts`
- Test: `web/src/i18n/teamNames.test.ts`

- [ ] **Step 1: Write the failing test**

Create `web/src/i18n/teamNames.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import type { Team } from '../graphql/types'
import { teamDisplayName } from './teamNames'

function team(partial: Partial<Team>): Team {
  return {
    id: 'X',
    name: 'English Name',
    shortCode: 'XXX',
    flag: null,
    ...partial,
  } as Team
}

describe('teamDisplayName', () => {
  it('returns the Hungarian name for a known team in hu', () => {
    const cro = team({ shortCode: 'CRO', name: 'Croatia' })
    expect(teamDisplayName(cro, 'hu')).toBe('Horvátország')
  })

  it('falls back to the English name in en (no en catalogue)', () => {
    const cro = team({ shortCode: 'CRO', name: 'Croatia' })
    expect(teamDisplayName(cro, 'en')).toBe('Croatia')
  })

  it('falls back to the English name for a team absent from the catalogue', () => {
    const unknown = team({ shortCode: 'ZZZ', name: 'Atlantis' })
    expect(teamDisplayName(unknown, 'hu')).toBe('Atlantis')
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- src/i18n/teamNames.test.ts`
Expected: FAIL — `Failed to resolve import "./teamNames"` (module does not exist yet).

- [ ] **Step 3: Write the catalogue + resolver**

Create `web/src/i18n/teamNames.ts`:

```ts
import type { Team } from '../graphql/types'
import type { Locale } from './strings'

/**
 * Localised team display names, keyed by `Team.shortCode` (the stable,
 * language-neutral identity). A locale with no entry for a team — or an omitted
 * locale such as `en` — falls back to the English name from the tournament JSON,
 * so the roster can change without touching this file.
 *
 * `en` is intentionally omitted: English names already come from `fwc26.json`.
 * The `hu` set covers the current FWC26 roster; correct wording here before
 * go-live.
 */
export const teamNames: Partial<Record<Locale, Record<string, string>>> = {
  hu: {
    ALG: 'Algéria',
    ARG: 'Argentína',
    AUS: 'Ausztrália',
    AUT: 'Ausztria',
    BEL: 'Belgium',
    BIH: 'Bosznia-Hercegovina',
    BRA: 'Brazília',
    CAN: 'Kanada',
    CIV: 'Elefántcsontpart',
    COD: 'Kongói DK',
    COL: 'Kolumbia',
    CPV: 'Zöld-foki Köztársaság',
    CRO: 'Horvátország',
    CUW: 'Curaçao',
    CZE: 'Csehország',
    ECU: 'Ecuador',
    EGY: 'Egyiptom',
    ENG: 'Anglia',
    ESP: 'Spanyolország',
    FRA: 'Franciaország',
    GER: 'Németország',
    GHA: 'Ghána',
    HAI: 'Haiti',
    IRN: 'Irán',
    IRQ: 'Irak',
    JOR: 'Jordánia',
    JPN: 'Japán',
    KOR: 'Dél-Korea',
    KSA: 'Szaúd-Arábia',
    MAR: 'Marokkó',
    MEX: 'Mexikó',
    NED: 'Hollandia',
    NOR: 'Norvégia',
    NZL: 'Új-Zéland',
    PAN: 'Panama',
    PAR: 'Paraguay',
    POR: 'Portugália',
    QAT: 'Katar',
    RSA: 'Dél-Afrika',
    SCO: 'Skócia',
    SEN: 'Szenegál',
    SUI: 'Svájc',
    SWE: 'Svédország',
    TUN: 'Tunézia',
    TUR: 'Törökország',
    URU: 'Uruguay',
    USA: 'USA',
    UZB: 'Üzbegisztán',
  },
}

/**
 * Resolve a team's display name for a locale: the localised name if present,
 * otherwise the English `team.name`. Never blank.
 */
export function teamDisplayName(team: Team, locale: Locale): string {
  return teamNames[locale]?.[team.shortCode] ?? team.name
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm run test -- src/i18n/teamNames.test.ts`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add web/src/i18n/teamNames.ts web/src/i18n/teamNames.test.ts
git commit -m "feat(web): add localised team-name catalogue + resolver"
```

---

## Task 2: Localise at the `teamIndex` boundary

**Files:**
- Modify: `web/src/lib/format.ts` (`teamIndex`)
- Test: `web/src/lib/format.test.ts`
- Modify call sites: `SchedulePage.tsx`, `TodayPage.tsx`, `PerfectPage.tsx`, `AllTipsPage.tsx`, `mytips/GroupTipForm.tsx`, `admin/AdminTeams.tsx`

- [ ] **Step 1: Write the failing test**

Add to `web/src/lib/format.test.ts` (append inside the file, top-level — add the import for `teamIndex` if not already imported):

```ts
import { teamIndex } from './format'

describe('teamIndex localisation', () => {
  const teams = [
    { id: 'CRO', name: 'Croatia', shortCode: 'CRO', flag: null },
    { id: 'ZZZ', name: 'Atlantis', shortCode: 'ZZZ', flag: null },
  ] as unknown as import('../graphql/types').Team[]

  it('localises names for hu, falling back for unknown teams', () => {
    const map = teamIndex(teams, 'hu')
    expect(map.get('CRO')?.name).toBe('Horvátország')
    expect(map.get('ZZZ')?.name).toBe('Atlantis')
  })

  it('keeps English names for en', () => {
    const map = teamIndex(teams, 'en')
    expect(map.get('CRO')?.name).toBe('Croatia')
  })

  it('returns new team objects (does not mutate input)', () => {
    const map = teamIndex(teams, 'hu')
    expect(map.get('CRO')).not.toBe(teams[0])
    expect(teams[0].name).toBe('Croatia')
  })
})
```

Note: if `format.test.ts` already imports `teamIndex` or `describe/expect/it`, do not duplicate those imports — reuse the existing ones.

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- src/lib/format.test.ts`
Expected: FAIL — `teamIndex` currently takes one argument; the localised names are not applied (`map.get('CRO')?.name` is `'Croatia'`, not `'Horvátország'`), and/or a TypeScript arity error.

- [ ] **Step 3: Update `teamIndex` to localise**

In `web/src/lib/format.ts`, change the import line and `teamIndex`:

Replace:

```ts
import type { SingleGame, Team, TeamSlot } from '../graphql/types'

/** Index teams by id for quick lookup. */
export function teamIndex(teams: Team[]): Map<string, Team> {
  return new Map(teams.map((t) => [t.id, t]))
}
```

With:

```ts
import type { SingleGame, Team, TeamSlot } from '../graphql/types'
import type { Locale } from '../i18n/strings'
import { teamDisplayName } from '../i18n/teamNames'

/**
 * Index teams by id for quick lookup, resolving each team's `name` to the
 * given locale's display name (English fallback). Localising here means every
 * downstream consumer — team labels, flag alt text, slot labels, admin sort —
 * is localised without further changes.
 */
export function teamIndex(teams: Team[], locale: Locale): Map<string, Team> {
  return new Map(
    teams.map((t) => [t.id, { ...t, name: teamDisplayName(t, locale) }]),
  )
}
```

- [ ] **Step 4: Run the resolver/index tests to verify they pass**

Run: `npm run test -- src/lib/format.test.ts`
Expected: PASS for the new `teamIndex localisation` block. (Other tests in the file are unaffected.)

- [ ] **Step 5: Update call site — `SchedulePage.tsx`**

`locale` is already destructured (`const { t, locale } = useI18n()`). Update the memo (around line 22-25):

Replace:
```tsx
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? []),
    [tournament],
  )
```
With:
```tsx
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? [], locale),
    [tournament, locale],
  )
```

- [ ] **Step 6: Update call site — `TodayPage.tsx`**

`locale` is already destructured (`const { t, locale } = useI18n()`). Update the memo (around line 64-67):

Replace:
```tsx
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? []),
    [tournament],
  )
```
With:
```tsx
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? [], locale),
    [tournament, locale],
  )
```

- [ ] **Step 7: Update call site — `PerfectPage.tsx`**

Add `locale` to the destructure: change `const { t } = useI18n()` to `const { t, locale } = useI18n()`. Then update the memo (around line 25-28):

Replace:
```tsx
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? []),
    [tournament],
  )
```
With:
```tsx
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? [], locale),
    [tournament, locale],
  )
```

- [ ] **Step 8: Update call site — `AllTipsPage.tsx`**

Add `locale` to the destructure: change `const { t } = useI18n()` to `const { t, locale } = useI18n()`. Then update the memo (around line 74-77):

Replace:
```tsx
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? []),
    [tournament?.teams],
  )
```
With:
```tsx
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? [], locale),
    [tournament?.teams, locale],
  )
```

- [ ] **Step 9: Update call site — `mytips/GroupTipForm.tsx`**

Add `locale` to the destructure: change `const { t } = useI18n()` to `const { t, locale } = useI18n()`. Then update the memo (around line 73):

Replace:
```tsx
  const teams = useMemo(() => teamIndex(tournament.teams), [tournament])
```
With:
```tsx
  const teams = useMemo(() => teamIndex(tournament.teams, locale), [tournament, locale])
```

- [ ] **Step 10: Update call site — `admin/AdminTeams.tsx`**

Add `locale` to the destructure: change `const { t } = useI18n()` to `const { t, locale } = useI18n()`. Then update the index build (around line 19):

Replace:
```tsx
  const teamMap = teamIndex(teams)
```
With:
```tsx
  const teamMap = teamIndex(teams, locale)
```

- [ ] **Step 11: Typecheck + full unit suite + lint**

Run: `npm run build`
Expected: PASS — no `tsc` arity errors at any call site.

Run: `npm run test`
Expected: PASS — all unit tests, including the new `teamIndex` block, green.

Run: `npm run lint`
Expected: no errors (no unused `locale`, exhaustive-deps satisfied).

- [ ] **Step 12: Commit**

```bash
git add web/src/lib/format.ts web/src/lib/format.test.ts \
  web/src/pages/SchedulePage.tsx web/src/pages/TodayPage.tsx \
  web/src/pages/PerfectPage.tsx web/src/pages/AllTipsPage.tsx \
  web/src/pages/mytips/GroupTipForm.tsx web/src/pages/admin/AdminTeams.tsx
git commit -m "feat(web): localise team names at the teamIndex boundary"
```

---

## Task 3: End-to-end verification (HU ↔ EN team names)

**Files:**
- Create: `web/e2e/team-names-i18n.spec.ts`

The e2e stack boots itself (`web/e2e/global-setup.ts`); `/games` is public. The
language picker is `.lang-selector select`; the team-display selector is
`.header-controls select`. Mexico (host) plays the opening match, so `MEX`
is always on the schedule — `Mexico` (en) / `Mexikó` (hu).

- [ ] **Step 1: Write the e2e test**

Create `web/e2e/team-names-i18n.spec.ts`:

```ts
import { test, expect } from '@playwright/test'
import { expectNoErrorView, watchNetwork } from './helpers'

/**
 * Team names follow the UI language. `/games` is public; we force the
 * name-showing display mode, then toggle the language picker and assert a known
 * team (Mexico — the host, always on the schedule) renders in Hungarian, then
 * back in English.
 */
test('team names render in the selected UI language', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/games')
  await expect(page.locator('h2')).toHaveText('Schedule')
  await expectNoErrorView(page)

  // Show full names (not flags/codes) so the text is assertable.
  await page.locator('.header-controls select').first().selectOption('name')

  const lang = page.locator('.lang-selector select')

  // English baseline.
  await lang.selectOption('en')
  await expect(page.locator('.team-label-text', { hasText: 'Mexico' }).first()).toBeVisible()

  // Hungarian: the same team renders localised; the English name is gone.
  await lang.selectOption('hu')
  await expect(page.locator('.team-label-text', { hasText: 'Mexikó' }).first()).toBeVisible()
  await expect(page.locator('.team-label-text', { hasText: /^Mexico$/ })).toHaveCount(0)

  // Back to English.
  await lang.selectOption('en')
  await expect(page.locator('.team-label-text', { hasText: 'Mexico' }).first()).toBeVisible()

  net.assertNoPageErrors()
  await net.assertNoGraphqlErrors()
})
```

- [ ] **Step 2: Run the e2e test**

Run: `npm run e2e -- team-names-i18n`
Expected: PASS (1 test). The harness boots docker + DynamoDB + API + Vite on the e2e profile automatically.

If the run fails because the language picker isn't on `/games` or the selector differs, inspect the rendered header (`web/src/components/LanguageSelector.tsx` renders `.lang-selector select`) and the page layout, and adjust the selector — do not weaken the assertion.

- [ ] **Step 3: Commit**

```bash
git add web/e2e/team-names-i18n.spec.ts
git commit -m "test(web): e2e — team names follow the UI language"
```

---

## Final verification

- [ ] **Run the full web checks from `web/`:**

```bash
npm run build   # tsc -b && vite build — no type errors
npm run test    # all unit tests green
npm run lint    # clean
npm run e2e -- team-names-i18n   # localisation e2e green
```

- [ ] **Manual spot check (optional):** `bin/local-dev localised-team-names`, open the app, switch language to Magyar, confirm team names on the Schedule / Today / All Tips pages render in Hungarian.

- [ ] **Finish the branch:** use superpowers:finishing-a-development-branch to merge `localised-team-names` into `master` (solo project — local merge; no PR needed unless you want the record).

---

## Notes for the implementer

- **Why localise at `teamIndex` and not in `teamLabelParts`?** `teamLabelParts` and `slotLabel` are pure and locale-unaware; localising once at the index keeps them (and their existing unit tests in `displayMode.test.ts`) untouched while covering every render path, flag alt text, and the admin sort.
- **`displayMode.test.ts` must stay green and unchanged** — it builds English team maps directly and never calls `teamIndex`.
- **Flags are unaffected** — assets are keyed by ISO/short code; only the text (and its `alt`/`title`) localises.
- **English needs no catalogue entries** — `teamNames.en` is omitted; English falls through to the JSON `name`.
