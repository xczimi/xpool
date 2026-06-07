# Theme Switcher Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add six selectable accent-colour presets and a System/Dark/Light mode toggle to the SPA chrome, persisted per-device.

**Architecture:** Two orthogonal axes — `data-accent` and `data-theme` — are mirrored onto `<html>` by a `ThemeProvider` (twin of the existing `DisplayModeProvider`). CSS attribute-selector blocks in `index.css` retheme by overriding custom properties; components are untouched because the accent spectrum (`--amber*`) is aliased to new semantic `--accent*` tokens. A `ThemeSelector` in `.header-controls` drives it; preferences persist to `localStorage`.

**Tech Stack:** React + TypeScript + Vite, urql; vitest (unit, `node` env), Playwright (e2e). CSS custom properties in `web/src/index.css`.

**Spec:** `docs/superpowers/specs/2026-06-07-theme-switcher-design.md`

**Working directory:** worktree `.claude/worktrees/theme-switcher` (branch `worktree-theme-switcher`). All commands below run from `web/` unless noted.

---

## File structure

| File | Responsibility |
|---|---|
| `web/src/theme/theme.ts` (create) | Pure core: `Accent`/`ThemeMode`/`ResolvedTheme` types, `ACCENTS`, `THEME_MODES`, defaults, storage keys, `coerceAccent`/`coerceThemeMode`, `resolveTheme`. No DOM, no React. |
| `web/src/theme/theme.test.ts` (create) | Unit tests for the pure core. |
| `web/src/theme/themeContextValue.ts` (create) | `ThemeContext` + `ThemeState` interface. |
| `web/src/theme/ThemeProvider.tsx` (create) | State, localStorage I/O, `matchMedia` wiring, `<html>` attribute effect. |
| `web/src/theme/useTheme.ts` (create) | Context hook (throws outside provider). |
| `web/src/components/ThemeSelector.tsx` (create) | Accent swatches + System/Dark/Light segmented control. |
| `web/src/index.css` (modify) | Semantic accent tokens + aliases, per-accent blocks, light-mode block, tokenise stray hex, soften light scanline, selector styles. |
| `web/src/i18n/strings.ts` (modify) | New `en` + `hu` keys for the control. |
| `web/src/components/Layout.tsx` (modify) | Mount `<ThemeSelector />` in `.header-controls`. |
| `web/src/main.tsx` (modify) | Wrap app in `<ThemeProvider>`. |
| `web/e2e/theme.spec.ts` (create) | Integration verification (visitor, no auth). |

---

## Task 1: Pure theme core (`theme.ts`)

**Files:**
- Create: `web/src/theme/theme.ts`
- Test: `web/src/theme/theme.test.ts`

- [ ] **Step 1: Write the failing test**

Create `web/src/theme/theme.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import {
  ACCENTS,
  THEME_MODES,
  coerceAccent,
  coerceThemeMode,
  resolveTheme,
} from './theme'

describe('ACCENTS', () => {
  it('lists the six accents, amber first', () => {
    expect(ACCENTS).toEqual([
      'amber',
      'green',
      'cyan',
      'magenta',
      'violet',
      'mono',
    ])
  })
})

describe('THEME_MODES', () => {
  it('lists system, dark, light', () => {
    expect(THEME_MODES).toEqual(['system', 'dark', 'light'])
  })
})

describe('coerceAccent', () => {
  it('passes through every valid accent', () => {
    for (const a of ACCENTS) expect(coerceAccent(a)).toBe(a)
  })
  it('falls back to amber on junk', () => {
    expect(coerceAccent('orange')).toBe('amber')
    expect(coerceAccent('')).toBe('amber')
    expect(coerceAccent(null)).toBe('amber')
    expect(coerceAccent(undefined)).toBe('amber')
  })
})

describe('coerceThemeMode', () => {
  it('passes through every valid mode', () => {
    for (const m of THEME_MODES) expect(coerceThemeMode(m)).toBe(m)
  })
  it('falls back to system on junk', () => {
    expect(coerceThemeMode('auto')).toBe('system')
    expect(coerceThemeMode('')).toBe('system')
    expect(coerceThemeMode(null)).toBe('system')
  })
})

describe('resolveTheme', () => {
  it('resolves system via the OS flag', () => {
    expect(resolveTheme('system', true)).toBe('dark')
    expect(resolveTheme('system', false)).toBe('light')
  })
  it('passes explicit modes through unchanged', () => {
    expect(resolveTheme('dark', false)).toBe('dark')
    expect(resolveTheme('light', true)).toBe('light')
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- theme`
Expected: FAIL — cannot resolve `./theme` (module does not exist yet).

- [ ] **Step 3: Write the minimal implementation**

Create `web/src/theme/theme.ts`:

```ts
/**
 * Accent colour presets and dark/light mode — pure logic (no DOM, no React).
 * Mirrors the split used by `lib/displayMode.ts`: the testable core lives here;
 * the React provider that applies it to the document is separate.
 */

/** Selectable accent hue. `amber` is the default / brand baseline. */
export type Accent = 'amber' | 'green' | 'cyan' | 'magenta' | 'violet' | 'mono'

/** What the user picks for mode. `system` follows the OS preference. */
export type ThemeMode = 'system' | 'dark' | 'light'

/** A mode with `system` already resolved to a concrete palette. */
export type ResolvedTheme = 'dark' | 'light'

/** Accent options, in display order. */
export const ACCENTS: readonly Accent[] = [
  'amber',
  'green',
  'cyan',
  'magenta',
  'violet',
  'mono',
]

/** Mode options, in display order. */
export const THEME_MODES: readonly ThemeMode[] = ['system', 'dark', 'light']

export const DEFAULT_ACCENT: Accent = 'amber'
export const DEFAULT_MODE: ThemeMode = 'system'

export const ACCENT_STORAGE_KEY = 'xpool.accent'
export const MODE_STORAGE_KEY = 'xpool.themeMode'

const ACCENT_SET: ReadonlySet<string> = new Set(ACCENTS)
const MODE_SET: ReadonlySet<string> = new Set(THEME_MODES)

/** Coerce an untrusted value to a valid Accent, else the default. */
export function coerceAccent(value: unknown): Accent {
  return typeof value === 'string' && ACCENT_SET.has(value)
    ? (value as Accent)
    : DEFAULT_ACCENT
}

/** Coerce an untrusted value to a valid ThemeMode, else the default. */
export function coerceThemeMode(value: unknown): ThemeMode {
  return typeof value === 'string' && MODE_SET.has(value)
    ? (value as ThemeMode)
    : DEFAULT_MODE
}

/** Resolve a mode to a concrete palette, consulting the OS only for `system`. */
export function resolveTheme(
  mode: ThemeMode,
  systemPrefersDark: boolean,
): ResolvedTheme {
  if (mode === 'system') {
    return systemPrefersDark ? 'dark' : 'light'
  }
  return mode
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- theme`
Expected: PASS (all describe blocks green).

- [ ] **Step 5: Commit**

```bash
git add web/src/theme/theme.ts web/src/theme/theme.test.ts
git commit -m "feat(web): pure accent/theme core + unit tests"
```

---

## Task 2: CSS token architecture + accent/light themes

**Files:**
- Modify: `web/src/index.css` (`:root` block ~lines 7–46; `.app-header` ~line 93; `.auth-bar` ~line 152; `body` scanline ~lines 49–66)

No unit test (CSS is not type-checked and the design system has no CSS tests). Verification is `npm run build` staying green + a manual check that **dark mode looks unchanged**. The visual confirmation of light mode / accents happens in the Task 6 e2e and manual review.

- [ ] **Step 1: Replace the amber spectrum with semantic accent tokens**

In `web/src/index.css`, find the amber spectrum in `:root`:

```css
  /* Amber spectrum (the primary brand colour) */
  --amber:          #ff8c00;
  --amber-bright:   #ffd76a;
  --amber-dim:      #a04400;
```

Replace it with:

```css
  /* Accent spectrum — the brand colour, themeable via [data-accent].
     Legacy --amber* names alias the accent so existing var(--amber*) callers
     (≈47 of them) retheme automatically with zero component churn. */
  --accent:         #ff8c00;
  --accent-bright:  #ffd76a;
  --accent-dim:     #a04400;
  --amber:          var(--accent);
  --amber-bright:   var(--accent-bright);
  --amber-dim:      var(--accent-dim);

  /* Bar surface — slightly darker than --bg-card (used by the auth bar). */
  --bg-bar:         #0a0a14;
```

- [ ] **Step 2: Invert the old `--accent` alias**

Still in `:root`, find the backwards-compat alias line:

```css
  --accent:       var(--amber);
```

Delete that single line (the `--accent` token is now the source of truth, defined in Step 1). Leave the neighbouring aliases (`--accent-ink: var(--bg-deep);` etc.) untouched.

- [ ] **Step 3: Tokenise the two stray hardcoded surfaces**

Find `.app-header` (~line 93):

```css
  background: linear-gradient(180deg, #0e0e1a 0%, #07070d 100%);
```

Replace with (identical in dark, themeable in light):

```css
  background: linear-gradient(180deg, var(--bg-card) 0%, var(--bg-deep) 100%);
```

Find `.auth-bar` (~line 152):

```css
  background: #0a0a14;
```

Replace with:

```css
  background: var(--bg-bar);
```

- [ ] **Step 4: Add per-accent preset blocks**

Immediately after the closing `}` of the `:root` block (the first one, ~line 46), add:

```css
/* ── Accent presets. `amber` is the :root baseline and needs no override. ── */
:root[data-accent='green']   { --accent: #33ff66; --accent-bright: #8effb0; --accent-dim: #14a33d; }
:root[data-accent='cyan']    { --accent: #21d4fd; --accent-bright: #8be9ff; --accent-dim: #0a7fa0; }
:root[data-accent='magenta'] { --accent: #ff49d0; --accent-bright: #ffa6ef; --accent-dim: #9c1f86; }
:root[data-accent='violet']  { --accent: #9d6bff; --accent-bright: #c9b0ff; --accent-dim: #5a3aa0; }
:root[data-accent='mono']    { --accent: #e8e8f0; --accent-bright: #ffffff; --accent-dim: #8a8a99; }
```

- [ ] **Step 5: Add the light-mode block + soften its scanline**

Directly below the accent blocks from Step 4, add:

```css
/* ── Light mode — lightweight inverted palette (warm paper, dark ink). ───── */
:root[data-theme='light'] {
  --bg-deep:        #f2eee1;
  --bg-card:        #fffdf6;
  --bg-input:       #ffffff;
  --bg-card-border: #d8c4a0;
  --bg-bar:         #ece6d4;
  --text-on-dark:   #1c1813;
  --text-dim:       #6a5a3a;
  --flash-bg:       #fdf3dc;
  --flash-border:   var(--accent-dim);
}

/* Soften the CRT scanline on light surfaces (the dark overlay is too heavy). */
:root[data-theme='light'] body {
  background-image:
    repeating-linear-gradient(
      0deg,
      rgba(0, 0, 0, 0.05) 0,
      rgba(0, 0, 0, 0.05) 1px,
      transparent 1px,
      transparent 4px
    );
}
```

- [ ] **Step 6: Verify the build is clean and dark mode is unchanged**

Run: `npm run build`
Expected: PASS (`tsc -b && vite build` exits 0).

Run: `npm run lint`
Expected: PASS.

Manual check (optional but recommended): `npm run dev`, confirm the app looks identical to before (no `data-*` attributes are set yet, so `:root` baseline applies).

- [ ] **Step 7: Commit**

```bash
git add web/src/index.css
git commit -m "feat(web): themeable accent tokens + light-mode palette in CSS"
```

---

## Task 3: Theme provider, context, hook + mount

**Files:**
- Create: `web/src/theme/themeContextValue.ts`
- Create: `web/src/theme/ThemeProvider.tsx`
- Create: `web/src/theme/useTheme.ts`
- Modify: `web/src/main.tsx`

This is React/DOM wiring; per the repo pattern (`DisplayModeProvider` is not unit-tested) it is verified by the Task 6 e2e. The gate here is a clean build.

- [ ] **Step 1: Create the context value**

Create `web/src/theme/themeContextValue.ts`:

```ts
import { createContext } from 'react'
import type { Accent, ResolvedTheme, ThemeMode } from './theme'

export interface ThemeState {
  accent: Accent
  mode: ThemeMode
  /** `mode` with `system` resolved to dark/light — what `data-theme` shows. */
  resolved: ResolvedTheme
  setAccent: (accent: Accent) => void
  setMode: (mode: ThemeMode) => void
}

export const ThemeContext = createContext<ThemeState | undefined>(undefined)
```

- [ ] **Step 2: Create the provider**

Create `web/src/theme/ThemeProvider.tsx`:

```tsx
import { useEffect, useMemo, useState, type ReactNode } from 'react'
import {
  ACCENT_STORAGE_KEY,
  MODE_STORAGE_KEY,
  coerceAccent,
  coerceThemeMode,
  resolveTheme,
  type Accent,
  type ThemeMode,
} from './theme'
import { ThemeContext, type ThemeState } from './themeContextValue'

const DARK_QUERY = '(prefers-color-scheme: dark)'

function readStored<T>(key: string, coerce: (v: unknown) => T): T {
  try {
    return coerce(localStorage.getItem(key))
  } catch {
    return coerce(null)
  }
}

function write(key: string, value: string): void {
  try {
    localStorage.setItem(key, value)
  } catch {
    /* ignore */
  }
}

/** Accent + dark/light theme preference — persisted to localStorage. */
export function ThemeProvider({ children }: { children: ReactNode }) {
  const [accent, setAccentState] = useState<Accent>(() =>
    readStored(ACCENT_STORAGE_KEY, coerceAccent),
  )
  const [mode, setModeState] = useState<ThemeMode>(() =>
    readStored(MODE_STORAGE_KEY, coerceThemeMode),
  )
  const [prefersDark, setPrefersDark] = useState<boolean>(
    () => window.matchMedia(DARK_QUERY).matches,
  )

  // Track the OS preference only while it can change the result (mode=system).
  useEffect(() => {
    if (mode !== 'system') return
    const mql = window.matchMedia(DARK_QUERY)
    const onChange = (e: MediaQueryListEvent) => setPrefersDark(e.matches)
    setPrefersDark(mql.matches)
    mql.addEventListener('change', onChange)
    return () => mql.removeEventListener('change', onChange)
  }, [mode])

  const resolved = resolveTheme(mode, prefersDark)

  // Mirror the resolved theme + accent onto <html> for the CSS to read.
  useEffect(() => {
    const root = document.documentElement
    root.setAttribute('data-accent', accent)
    root.setAttribute('data-theme', resolved)
  }, [accent, resolved])

  const value = useMemo<ThemeState>(
    () => ({
      accent,
      mode,
      resolved,
      setAccent: (next: Accent) => {
        write(ACCENT_STORAGE_KEY, next)
        setAccentState(next)
      },
      setMode: (next: ThemeMode) => {
        write(MODE_STORAGE_KEY, next)
        setModeState(next)
      },
    }),
    [accent, mode, resolved],
  )

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
}
```

- [ ] **Step 3: Create the hook**

Create `web/src/theme/useTheme.ts`:

```ts
import { useContext } from 'react'
import { ThemeContext, type ThemeState } from './themeContextValue'

export function useTheme(): ThemeState {
  const ctx = useContext(ThemeContext)
  if (!ctx) {
    throw new Error('useTheme must be used within ThemeProvider')
  }
  return ctx
}
```

- [ ] **Step 4: Mount the provider in `main.tsx`**

In `web/src/main.tsx`, add the import alongside the other provider imports:

```tsx
import { ThemeProvider } from './theme/ThemeProvider'
```

Then wrap the tree — place `<ThemeProvider>` just inside `<Auth0Gate>`, around `<I18nProvider>`:

```tsx
    <Auth0Gate>
      <ThemeProvider>
        <I18nProvider>
          <DisplayModeProvider>
            <AuthProvider>
              <GraphqlProvider>
                <BrowserRouter>
                  <App />
                </BrowserRouter>
              </GraphqlProvider>
            </AuthProvider>
          </DisplayModeProvider>
        </I18nProvider>
      </ThemeProvider>
    </Auth0Gate>
```

- [ ] **Step 5: Verify the build**

Run: `npm run build`
Expected: PASS. (The provider now sets `data-accent`/`data-theme`; with no UI yet they sit at the persisted/default values — amber + resolved-system.)

- [ ] **Step 6: Commit**

```bash
git add web/src/theme/themeContextValue.ts web/src/theme/ThemeProvider.tsx web/src/theme/useTheme.ts web/src/main.tsx
git commit -m "feat(web): ThemeProvider applies accent/theme to <html>"
```

---

## Task 4: i18n strings (en + hu)

**Files:**
- Modify: `web/src/i18n/strings.ts` (the `en` object ~line 19; the `hu` object ~line 189)

- [ ] **Step 1: Add the English keys**

In `web/src/i18n/strings.ts`, in the `en` object, find the chrome block ending at `displayFlagCode: 'Flag + code',`. Immediately after that line, add:

```ts
  theme: 'Theme',
  mode: 'Mode',
  accentAmber: 'Amber',
  accentGreen: 'Green',
  accentCyan: 'Cyan',
  accentMagenta: 'Magenta',
  accentViolet: 'Violet',
  accentMono: 'Mono',
  modeSystem: 'System',
  modeDark: 'Dark',
  modeLight: 'Light',
```

- [ ] **Step 2: Add the matching Hungarian keys**

In the `hu` object, find `displayFlagCode: 'Zászló + kód',`. Immediately after that line, add:

```ts
  theme: 'Téma',
  mode: 'Mód',
  accentAmber: 'Borostyán',
  accentGreen: 'Zöld',
  accentCyan: 'Cián',
  accentMagenta: 'Magenta',
  accentViolet: 'Lila',
  accentMono: 'Mono',
  modeSystem: 'Rendszer',
  modeDark: 'Sötét',
  modeLight: 'Világos',
```

- [ ] **Step 3: Verify the build (type-checks the catalogue completeness)**

Run: `npm run build`
Expected: PASS. `hu` is typed `Record<StringKey, string>`, so a missing key would fail `tsc`. Green means both catalogues match.

- [ ] **Step 4: Commit**

```bash
git add web/src/i18n/strings.ts
git commit -m "feat(web): i18n strings for the theme selector (en + hu)"
```

---

## Task 5: ThemeSelector component + chrome mount + styles

**Files:**
- Create: `web/src/components/ThemeSelector.tsx`
- Modify: `web/src/components/Layout.tsx`
- Modify: `web/src/index.css` (add selector styles near `.lang-selector` / `.header-controls`)

- [ ] **Step 1: Create the component**

Create `web/src/components/ThemeSelector.tsx`:

```tsx
import { useI18n } from '../i18n/useI18n'
import { useTheme } from '../theme/useTheme'
import { ACCENTS, THEME_MODES, type Accent, type ThemeMode } from '../theme/theme'
import type { StringKey } from '../i18n/strings'

const ACCENT_LABEL: Record<Accent, StringKey> = {
  amber: 'accentAmber',
  green: 'accentGreen',
  cyan: 'accentCyan',
  magenta: 'accentMagenta',
  violet: 'accentViolet',
  mono: 'accentMono',
}

const MODE_LABEL: Record<ThemeMode, StringKey> = {
  system: 'modeSystem',
  dark: 'modeDark',
  light: 'modeLight',
}

export function ThemeSelector() {
  const { t } = useI18n()
  const { accent, mode, setAccent, setMode } = useTheme()
  return (
    <div className="theme-selector">
      <div className="accent-swatches" role="radiogroup" aria-label={t('theme')}>
        {ACCENTS.map((a) => (
          <button
            key={a}
            type="button"
            role="radio"
            aria-checked={a === accent}
            aria-label={t(ACCENT_LABEL[a])}
            title={t(ACCENT_LABEL[a])}
            className={`accent-swatch accent-swatch-${a}${a === accent ? ' is-active' : ''}`}
            onClick={() => setAccent(a)}
          />
        ))}
      </div>
      <div className="mode-toggle" role="radiogroup" aria-label={t('mode')}>
        {THEME_MODES.map((m) => (
          <button
            key={m}
            type="button"
            role="radio"
            aria-checked={m === mode}
            className={`mode-option${m === mode ? ' is-active' : ''}`}
            onClick={() => setMode(m)}
          >
            {t(MODE_LABEL[m])}
          </button>
        ))}
      </div>
    </div>
  )
}
```

- [ ] **Step 2: Mount it in the chrome**

In `web/src/components/Layout.tsx`, add the import alongside the other component imports:

```tsx
import { ThemeSelector } from './ThemeSelector'
```

Then in the `.header-controls` block, add `<ThemeSelector />` after `<LanguageSelector />`:

```tsx
        <div className="header-controls">
          <DisplayModeSelector />
          <LanguageSelector />
          <ThemeSelector />
        </div>
```

- [ ] **Step 3: Add the selector styles**

In `web/src/index.css`, find the `.header-controls` rule (near the end of the file) and add the following directly after it:

```css
.theme-selector {
  display: flex;
  align-items: center;
  gap: 12px;
}

.accent-swatches {
  display: flex;
  gap: 6px;
}

.accent-swatch {
  width: 16px;
  height: 16px;
  padding: 0;
  border: 1px solid var(--bg-card-border);
  border-radius: 50%;
  cursor: pointer;
}

.accent-swatch.is-active {
  border-color: var(--text-on-dark);
  box-shadow: 0 0 0 2px var(--bg-deep), 0 0 6px currentColor;
}

.accent-swatch-amber   { background: #ff8c00; color: #ff8c00; }
.accent-swatch-green   { background: #33ff66; color: #33ff66; }
.accent-swatch-cyan    { background: #21d4fd; color: #21d4fd; }
.accent-swatch-magenta { background: #ff49d0; color: #ff49d0; }
.accent-swatch-violet  { background: #9d6bff; color: #9d6bff; }
.accent-swatch-mono    { background: #e8e8f0; color: #e8e8f0; }

.mode-toggle {
  display: flex;
  border: 1px solid var(--bg-card-border);
  border-radius: 4px;
  overflow: hidden;
}

.mode-option {
  font-family: 'Share Tech Mono', monospace;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 1px;
  padding: 3px 8px;
  background: transparent;
  color: var(--text-dim);
  border: none;
  cursor: pointer;
}

.mode-option.is-active {
  background: var(--accent);
  color: var(--accent-ink);
}
```

(The `color` on each swatch feeds the `currentColor` glow on the active ring.)

- [ ] **Step 4: Verify build + lint**

Run: `npm run build`
Expected: PASS.

Run: `npm run lint`
Expected: PASS.

- [ ] **Step 5: Manual smoke (optional)**

Run: `npm run dev`, open the app. The chrome shows six colour dots + a System/Dark/Light toggle. Click a dot → accent recolours everywhere. Click Light → palette inverts. Reload → choices stick.

- [ ] **Step 6: Commit**

```bash
git add web/src/components/ThemeSelector.tsx web/src/components/Layout.tsx web/src/index.css
git commit -m "feat(web): theme selector control in the chrome"
```

---

## Task 6: End-to-end verification

**Files:**
- Create: `web/e2e/theme.spec.ts`

- [ ] **Step 1: Write the e2e spec**

Create `web/e2e/theme.spec.ts`:

```ts
import { test, expect } from '@playwright/test'
import { expectNoErrorView, watchNetwork } from './helpers'

/**
 * Theme switcher — accent presets + dark/light mode. Both are global
 * localStorage preferences available to logged-out visitors; `/games` is
 * public, so no login is needed. We assert the <html> data-attributes drive
 * the CSS custom properties and that choices survive a reload.
 */
test('accent + mode switch, drive CSS tokens, and persist', async ({ page }) => {
  const net = watchNetwork(page)
  await page.goto('/games')
  await expect(page.locator('h2')).toHaveText('Schedule')
  await expectNoErrorView(page)

  const html = page.locator('html')
  const cssVar = (name: string) =>
    page.evaluate(
      (n) =>
        getComputedStyle(document.documentElement).getPropertyValue(n).trim(),
      name,
    )

  // Default accent is amber, and the token resolves to the amber base.
  await expect(html).toHaveAttribute('data-accent', 'amber')
  expect(await cssVar('--accent')).toBe('#ff8c00')

  // Pick the cyan accent → attribute + token both change.
  await page.getByRole('radio', { name: 'Cyan' }).click()
  await expect(html).toHaveAttribute('data-accent', 'cyan')
  expect(await cssVar('--accent')).toBe('#21d4fd')

  // Force Dark, capture the surface, then switch to Light → surface changes.
  await page.getByRole('radio', { name: 'Dark' }).click()
  await expect(html).toHaveAttribute('data-theme', 'dark')
  const darkBg = await cssVar('--bg-deep')

  await page.getByRole('radio', { name: 'Light' }).click()
  await expect(html).toHaveAttribute('data-theme', 'light')
  expect(await cssVar('--bg-deep')).not.toBe(darkBg)

  // Both choices persist across a reload.
  await page.reload()
  await expect(page.locator('h2')).toHaveText('Schedule')
  await expect(html).toHaveAttribute('data-accent', 'cyan')
  await expect(html).toHaveAttribute('data-theme', 'light')

  net.assertNoPageErrors()
  await net.assertNoGraphqlErrors()
})

/**
 * System mode follows the OS preference live, via the provider's matchMedia
 * subscription. Playwright's emulateMedia drives prefers-color-scheme.
 */
test('system mode follows prefers-color-scheme', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'light' })
  await page.goto('/games')
  await expect(page.locator('h2')).toHaveText('Schedule')

  const html = page.locator('html')

  // Default mode is system; emulated light OS preference → resolved light.
  await page.getByRole('radio', { name: 'System' }).click()
  await expect(html).toHaveAttribute('data-theme', 'light')

  // Flip the OS preference → the resolved theme follows without a reload.
  await page.emulateMedia({ colorScheme: 'dark' })
  await expect(html).toHaveAttribute('data-theme', 'dark')
})
```

- [ ] **Step 2: Run the e2e spec**

Run: `npm run e2e -- theme.spec.ts`
Expected: PASS (2 tests). The Playwright config boots its own stack via `e2e/global-setup.ts`.

Note: if the run fails because no auth is configured, it should **not** apply here (these tests never log in). If a flash of the default accent is observed before attributes apply, that is the deferred anti-flash item — only act on it if a test actually fails on timing; add `await expect(html).toHaveAttribute('data-accent', ...)` waits (already present) which tolerate the mount delay.

- [ ] **Step 3: Commit**

```bash
git add web/e2e/theme.spec.ts
git commit -m "test(web): e2e for accent + dark/light theme switching"
```

---

## Final verification (run before declaring done)

- [ ] `npm test` — vitest green (includes `theme.test.ts`).
- [ ] `npm run build` — `tsc -b && vite build` exits 0.
- [ ] `npm run lint` — eslint clean.
- [ ] `npm run e2e -- theme.spec.ts` — green.
- [ ] Manual: all six accents × dark/light render legibly; dark mode visually unchanged from before for the `amber` default; no dark surfaces leak into light mode (the two tokenised hex).

## Notes for the implementer

- **Immutability:** all state updates create new values (`setAccentState(next)`); no mutation. Follow the existing provider style exactly.
- **Why aliases, not a rename:** `--amber*` is referenced ~47 times across `index.css`. Aliasing to `--accent*` rethemes every consumer with no churn and keeps the diff reviewable. Do not mass-rename `var(--amber)` callers.
- **Default `data-theme` in CI:** the first test deliberately does not assert a default `data-theme` value — it depends on the runner's `prefers-color-scheme`. It forces Dark explicitly before comparing surfaces, so it is deterministic.
- **Out of scope (seams left):** language-picker segmented redesign, per-account profile sync, fully contrast-audited light theme, pre-hydration anti-flash script.
```
