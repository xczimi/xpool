# Country Flags + Display-Mode Toggle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show an 8-bit retro flag for every country and let the user switch how teams are displayed app-wide (flag / code / name / flag+name / flag+code, plus a responsive `auto` default).

**Architecture:** The domain `Team.flag` field (already wired through storage → GraphQL → SPA, currently `null`) is repurposed to hold the ISO-3166-1 alpha-2 country code; tiny PNG flags are bundled under `web/public/flags/` and rendered scaled-up with `image-rendering: pixelated`. A React context (mirroring the existing i18n provider) holds the chosen `DisplayMode` in `localStorage`; a pure resolver turns it into a concrete rendering, and a new `<TeamLabel>` component replaces the inline `slotLabel`/`slotCode` string calls at every render site.

**Tech Stack:** Rust workspace (data only — no code change), React 19 + Vite + TypeScript SPA, Vitest (pure-logic unit tests), Playwright (e2e). `jq` for the data edit, flagcdn.com for public-domain flag PNGs.

**Spec:** `docs/superpowers/specs/2026-06-06-country-flags-display-mode-design.md`

**Worktree:** `.claude/worktrees/country-flags` (branch `feat/country-flags`). All paths below are relative to that worktree root.

---

## File Structure

**Created:**
- `bin/fetch-flags` — reproducible downloader: reads ISO codes from `tournaments/fwc26.json`, fetches PNGs into `web/public/flags/`.
- `web/public/flags/<iso2>.png` — 48 bundled flag PNGs (committed).
- `web/src/lib/displayMode.ts` — pure logic: `DisplayMode` types, `resolveDisplayMode`, `teamLabelParts`. (In `src/lib/` so it's in the Vitest coverage scope.)
- `web/src/lib/displayMode.test.ts` — unit tests for the pure logic.
- `web/src/display/displayModeContextValue.ts` — context + `DisplayModeState` interface.
- `web/src/display/DisplayModeProvider.tsx` — provider with `localStorage` persistence.
- `web/src/display/useDisplayMode.ts` — context hook.
- `web/src/display/useResolvedDisplayMode.ts` — resolves `auto` via a `matchMedia` hook.
- `web/src/components/TeamLabel.tsx` — `<Flag>` leaf + `<TeamLabel>` slot renderer.
- `web/src/components/DisplayModeSelector.tsx` — header `<select>`.
- `web/e2e/team-display.spec.ts` — e2e for the toggle.

**Modified:**
- `tournaments/fwc26.json` — populate `flag` with ISO codes (48 teams).
- `web/src/i18n/strings.ts` — selector label + option strings (en + hu).
- `web/src/main.tsx` — wrap app in `DisplayModeProvider`.
- `web/src/components/Layout.tsx` — add `<DisplayModeSelector/>` to the header.
- `web/src/index.css` — `.team-label` / `.team-flag` / `.header-controls` styles.
- Render sites: `web/src/pages/SchedulePage.tsx`, `web/src/pages/TodayPage.tsx`, `web/src/pages/AllTipsPage.tsx`, `web/src/pages/mytips/StandingsTables.tsx`, `web/src/pages/PerfectPage.tsx`, `web/src/pages/admin/AdminResults.tsx`, `web/src/pages/admin/AdminTeams.tsx`.
- `.specs/DATA_MODEL.md` — document the `flag`-field repurposing.

---

## Task 1: Tournament data — ISO codes, fetch script, bundled PNGs

**Files:**
- Modify: `tournaments/fwc26.json`
- Create: `bin/fetch-flags`
- Create: `web/public/flags/*.png`

- [ ] **Step 1: Populate `flag` with ISO-3166-1 alpha-2 codes**

Run this `jq` command from the worktree root. The map covers all 48 fwc26 teams (England/Scotland use flagcdn's GB-subdivision codes):

```bash
jq --indent 2 '
  ($ARGS.named.map) as $m
  | .teams |= map(.flag = ($m[.id] // .flag))
' tournaments/fwc26.json --argjson map '{
  "MEX":"mx","RSA":"za","KOR":"kr","CZE":"cz","CAN":"ca","BIH":"ba","QAT":"qa",
  "SUI":"ch","BRA":"br","MAR":"ma","HAI":"ht","SCO":"gb-sct","USA":"us","PAR":"py",
  "AUS":"au","TUR":"tr","GER":"de","CUW":"cw","CIV":"ci","ECU":"ec","NED":"nl",
  "JPN":"jp","SWE":"se","TUN":"tn","ESP":"es","CPV":"cv","KSA":"sa","URU":"uy",
  "BEL":"be","EGY":"eg","IRN":"ir","NZL":"nz","FRA":"fr","SEN":"sn","IRQ":"iq",
  "NOR":"no","ARG":"ar","ALG":"dz","AUT":"at","JOR":"jo","POR":"pt","COD":"cd",
  "UZB":"uz","COL":"co","ENG":"gb-eng","CRO":"hr","GHA":"gh","PAN":"pa"
}' > tournaments/fwc26.json.tmp && mv tournaments/fwc26.json.tmp tournaments/fwc26.json
```

- [ ] **Step 2: Verify every team got a flag code**

Run: `jq '[.teams[] | select(.flag == null)] | length' tournaments/fwc26.json`
Expected: `0`

Run: `jq -r '.teams[] | "\(.id) \(.flag)"' tournaments/fwc26.json | head -3`
Expected (spot check): `MEX mx`, `RSA za`, `KOR kr`

- [ ] **Step 3: Create the flag-fetch script**

Create `bin/fetch-flags`:

```bash
#!/usr/bin/env bash
# Download 8-bit-ready flag PNGs for every country in fwc26.json.
#
# Source: flagcdn.com (public-domain flags, based on flagpedia). We fetch the
# smallest size (20x15) on purpose — the SPA scales them up with
# `image-rendering: pixelated`, so a tiny source gives the chunky retro look.
# PNGs are committed to the repo so dev and e2e never hit the network.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
data="$root/tournaments/fwc26.json"
out="$root/web/public/flags"
mkdir -p "$out"

mapfile -t isos < <(jq -r '.teams[].flag | select(. != null)' "$data" | sort -u)
echo "Fetching ${#isos[@]} flags into $out"

for iso in "${isos[@]}"; do
  dest="$out/$iso.png"
  if [[ -f "$dest" ]]; then
    echo "  skip $iso (exists)"
    continue
  fi
  url="https://flagcdn.com/20x15/$iso.png"
  echo "  get  $iso  <- $url"
  curl -fsSL "$url" -o "$dest"
done

echo "Done."
```

Then make it executable:

Run: `chmod +x bin/fetch-flags`

- [ ] **Step 4: Run the fetch and verify the assets**

Run: `bin/fetch-flags`
Expected: lines `get <iso> <- https://flagcdn.com/20x15/<iso>.png` for each flag, ending `Done.`

Run: `ls web/public/flags | wc -l`
Expected: `48`

Run: `file web/public/flags/br.png`
Expected: contains `PNG image data`

- [ ] **Step 5: Commit**

```bash
git add tournaments/fwc26.json bin/fetch-flags web/public/flags
git commit -m "feat(data): ISO flag codes for fwc26 teams + bundled 8-bit flag PNGs"
```

---

## Task 2: Pure display-mode logic (TDD)

**Files:**
- Create: `web/src/lib/displayMode.ts`
- Test: `web/src/lib/displayMode.test.ts`

- [ ] **Step 1: Write the failing test**

Create `web/src/lib/displayMode.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import type { Team, TeamSlot } from '../graphql/types'
import {
  DISPLAY_MODES,
  resolveDisplayMode,
  teamLabelParts,
} from './displayMode'

function team(over: Partial<Team>): Team {
  return {
    id: 'BRA',
    name: 'Brazil',
    shortCode: 'BRA',
    flag: 'br',
    externalId: null,
    ...over,
  }
}

function slot(over: Partial<TeamSlot>): TeamSlot {
  return { teamId: null, description: '', ...over }
}

const teams = new Map<string, Team>([['BRA', team({})]])
const noFlag = new Map<string, Team>([['BRA', team({ flag: null })]])

describe('DISPLAY_MODES', () => {
  it('lists auto first, then the five explicit modes', () => {
    expect(DISPLAY_MODES).toEqual([
      'auto',
      'flag',
      'code',
      'name',
      'flag-name',
      'flag-code',
    ])
  })
})

describe('resolveDisplayMode', () => {
  it('resolves auto to flag on mobile', () => {
    expect(resolveDisplayMode('auto', true)).toBe('flag')
  })
  it('resolves auto to flag-name on larger screens', () => {
    expect(resolveDisplayMode('auto', false)).toBe('flag-name')
  })
  it('passes explicit modes through unchanged', () => {
    for (const m of ['flag', 'code', 'name', 'flag-name', 'flag-code'] as const) {
      expect(resolveDisplayMode(m, true)).toBe(m)
      expect(resolveDisplayMode(m, false)).toBe(m)
    }
  })
})

describe('teamLabelParts', () => {
  const braSlot = slot({ teamId: 'BRA' })

  it('name mode: full name, no flag', () => {
    expect(teamLabelParts(braSlot, teams, 'name')).toEqual({
      flag: null,
      text: 'Brazil',
    })
  })
  it('code mode: short code, no flag', () => {
    expect(teamLabelParts(braSlot, teams, 'code')).toEqual({
      flag: null,
      text: 'BRA',
    })
  })
  it('flag mode: flag only, no text', () => {
    expect(teamLabelParts(braSlot, teams, 'flag')).toEqual({
      flag: { iso: 'br', name: 'Brazil' },
      text: null,
    })
  })
  it('flag-name mode: flag and name', () => {
    expect(teamLabelParts(braSlot, teams, 'flag-name')).toEqual({
      flag: { iso: 'br', name: 'Brazil' },
      text: 'Brazil',
    })
  })
  it('flag-code mode: flag and code', () => {
    expect(teamLabelParts(braSlot, teams, 'flag-code')).toEqual({
      flag: { iso: 'br', name: 'Brazil' },
      text: 'BRA',
    })
  })
  it('flag mode with no flag asset falls back to the code', () => {
    expect(teamLabelParts(braSlot, noFlag, 'flag')).toEqual({
      flag: null,
      text: 'BRA',
    })
  })
  it('flag-name mode with no flag asset still shows the name', () => {
    expect(teamLabelParts(braSlot, noFlag, 'flag-name')).toEqual({
      flag: null,
      text: 'Brazil',
    })
  })
  it('unresolved slot shows its placeholder description in every mode', () => {
    const ph = slot({ teamId: null, description: '2A' })
    expect(teamLabelParts(ph, teams, 'flag')).toEqual({ flag: null, text: '2A' })
    expect(teamLabelParts(ph, teams, 'name')).toEqual({ flag: null, text: '2A' })
  })
  it('unknown team id falls back to the id text', () => {
    const unknown = slot({ teamId: 'ZZZ' })
    expect(teamLabelParts(unknown, teams, 'flag-name')).toEqual({
      flag: null,
      text: 'ZZZ',
    })
  })
})
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd web && npm test -- displayMode`
Expected: FAIL — `Cannot find module './displayMode'` (or "is not a function").

- [ ] **Step 3: Write the implementation**

Create `web/src/lib/displayMode.ts`:

```ts
import type { Team, TeamSlot } from '../graphql/types'

/** How a team is displayed. `auto` resolves responsively (see resolveDisplayMode). */
export type DisplayMode =
  | 'auto'
  | 'flag'
  | 'code'
  | 'name'
  | 'flag-name'
  | 'flag-code'

/** A display mode with `auto` already resolved to a concrete rendering. */
export type ConcreteDisplayMode = Exclude<DisplayMode, 'auto'>

/** Selector options, in display order. */
export const DISPLAY_MODES: DisplayMode[] = [
  'auto',
  'flag',
  'code',
  'name',
  'flag-name',
  'flag-code',
]

/**
 * Resolve `auto` against the viewport: flag-only on mobile, flag + name on
 * larger screens. Every explicit mode passes through unchanged.
 */
export function resolveDisplayMode(
  mode: DisplayMode,
  isMobile: boolean,
): ConcreteDisplayMode {
  if (mode === 'auto') {
    return isMobile ? 'flag' : 'flag-name'
  }
  return mode
}

/** A flag image reference — the ISO code drives the asset path, name is alt text. */
export interface FlagPart {
  iso: string
  name: string
}

/** What to render for one team slot, already resolved for the current mode. */
export interface TeamLabelParts {
  flag: FlagPart | null
  text: string | null
}

const FLAG_MODES: ReadonlySet<ConcreteDisplayMode> = new Set([
  'flag',
  'flag-name',
  'flag-code',
])

/**
 * Decide the flag + text to show for a slot under a concrete mode.
 *
 * - Unresolved slots (no team yet) show their placeholder description in every
 *   mode, never a flag.
 * - When a flag is wanted but the team has no ISO code, fall back to text so a
 *   slot never renders empty.
 */
export function teamLabelParts(
  slot: TeamSlot,
  teams: Map<string, Team>,
  mode: ConcreteDisplayMode,
): TeamLabelParts {
  const team = slot.teamId ? teams.get(slot.teamId) : undefined

  // Unresolved slot, or an id we don't know — placeholder/text only.
  if (!team) {
    const fallback = slot.teamId ?? slot.description ?? 'TBD'
    return { flag: null, text: fallback || 'TBD' }
  }

  const wantsFlag = FLAG_MODES.has(mode)
  const flag =
    wantsFlag && team.flag ? { iso: team.flag, name: team.name } : null

  let text: string | null = null
  if (mode === 'name' || mode === 'flag-name') {
    text = team.name
  } else if (mode === 'code' || mode === 'flag-code') {
    text = team.shortCode
  } else if (mode === 'flag') {
    // Flag-only — but if the flag asset is missing, fall back to the code.
    text = flag ? null : team.shortCode
  }

  return { flag, text }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd web && npm test -- displayMode`
Expected: PASS (all cases green).

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/displayMode.ts web/src/lib/displayMode.test.ts
git commit -m "feat(web): pure display-mode resolver + team-label parts"
```

---

## Task 3: Display-mode context, provider & hooks

**Files:**
- Create: `web/src/display/displayModeContextValue.ts`
- Create: `web/src/display/DisplayModeProvider.tsx`
- Create: `web/src/display/useDisplayMode.ts`
- Create: `web/src/display/useResolvedDisplayMode.ts`
- Modify: `web/src/main.tsx`

- [ ] **Step 1: Create the context value**

Create `web/src/display/displayModeContextValue.ts`:

```ts
import { createContext } from 'react'
import type { DisplayMode } from '../lib/displayMode'

export interface DisplayModeState {
  mode: DisplayMode
  setMode: (mode: DisplayMode) => void
}

export const DisplayModeContext = createContext<DisplayModeState | undefined>(
  undefined,
)
```

- [ ] **Step 2: Create the provider**

Create `web/src/display/DisplayModeProvider.tsx` (mirrors `I18nProvider`):

```tsx
import { useMemo, useState, type ReactNode } from 'react'
import { DISPLAY_MODES, type DisplayMode } from '../lib/displayMode'
import {
  DisplayModeContext,
  type DisplayModeState,
} from './displayModeContextValue'

const STORAGE_KEY = 'xpool.displayMode'

const VALID: ReadonlySet<string> = new Set(DISPLAY_MODES)

function initialMode(): DisplayMode {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored && VALID.has(stored)) {
      return stored as DisplayMode
    }
  } catch {
    /* ignore */
  }
  return 'auto'
}

/** Team display-mode preference — persisted to localStorage. */
export function DisplayModeProvider({ children }: { children: ReactNode }) {
  const [mode, setModeState] = useState<DisplayMode>(initialMode())

  const value = useMemo<DisplayModeState>(
    () => ({
      mode,
      setMode: (next: DisplayMode) => {
        try {
          localStorage.setItem(STORAGE_KEY, next)
        } catch {
          /* ignore */
        }
        setModeState(next)
      },
    }),
    [mode],
  )

  return (
    <DisplayModeContext.Provider value={value}>
      {children}
    </DisplayModeContext.Provider>
  )
}
```

- [ ] **Step 3: Create the context hook**

Create `web/src/display/useDisplayMode.ts`:

```ts
import { useContext } from 'react'
import {
  DisplayModeContext,
  type DisplayModeState,
} from './displayModeContextValue'

export function useDisplayMode(): DisplayModeState {
  const ctx = useContext(DisplayModeContext)
  if (!ctx) {
    throw new Error('useDisplayMode must be used within DisplayModeProvider')
  }
  return ctx
}
```

- [ ] **Step 4: Create the resolved-mode hook**

Create `web/src/display/useResolvedDisplayMode.ts`. The `640px` breakpoint matches the existing mobile breakpoint in `index.css`:

```ts
import { useEffect, useState } from 'react'
import {
  resolveDisplayMode,
  type ConcreteDisplayMode,
} from '../lib/displayMode'
import { useDisplayMode } from './useDisplayMode'

const MOBILE_QUERY = '(max-width: 640px)'

/** Track the mobile media query, updating live on resize/rotate. */
function useIsMobile(): boolean {
  const [isMobile, setIsMobile] = useState(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return false
    return window.matchMedia(MOBILE_QUERY).matches
  })

  useEffect(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return
    const mql = window.matchMedia(MOBILE_QUERY)
    const onChange = (e: MediaQueryListEvent) => setIsMobile(e.matches)
    setIsMobile(mql.matches)
    mql.addEventListener('change', onChange)
    return () => mql.removeEventListener('change', onChange)
  }, [])

  return isMobile
}

/** The current display mode with `auto` resolved against the viewport. */
export function useResolvedDisplayMode(): ConcreteDisplayMode {
  const { mode } = useDisplayMode()
  const isMobile = useIsMobile()
  return resolveDisplayMode(mode, isMobile)
}
```

- [ ] **Step 5: Wrap the app in the provider**

Modify `web/src/main.tsx`. Add the import after the `I18nProvider` import:

```tsx
import { I18nProvider } from './i18n/I18nContext'
import { DisplayModeProvider } from './display/DisplayModeProvider'
```

Then wrap inside `I18nProvider` (so the selector, which uses both contexts, is covered). Replace:

```tsx
      <I18nProvider>
        <AuthProvider>
```

with:

```tsx
      <I18nProvider>
        <DisplayModeProvider>
          <AuthProvider>
```

and the matching closing tags — replace:

```tsx
        </AuthProvider>
      </I18nProvider>
```

with:

```tsx
          </AuthProvider>
        </DisplayModeProvider>
      </I18nProvider>
```

- [ ] **Step 6: Verify it type-checks**

Run: `cd web && npx tsc -b`
Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add web/src/display web/src/main.tsx
git commit -m "feat(web): display-mode context, provider & resolved-mode hook"
```

---

## Task 4: i18n strings for the selector

**Files:**
- Modify: `web/src/i18n/strings.ts`

- [ ] **Step 1: Add the English strings**

In `web/src/i18n/strings.ts`, in the `const en = {` block, find the `// chrome` line `language: 'Language',` and add directly after it:

```ts
  display: 'Show',
  displayAuto: 'Auto',
  displayFlag: 'Flag',
  displayCode: 'Code',
  displayName: 'Name',
  displayFlagName: 'Flag + name',
  displayFlagCode: 'Flag + code',
```

- [ ] **Step 2: Add the Hungarian strings**

In the `const hu: Record<StringKey, string> = {` block, find `language: 'Nyelv',` and add directly after it:

```ts
  display: 'Mutat',
  displayAuto: 'Auto',
  displayFlag: 'Zászló',
  displayCode: 'Kód',
  displayName: 'Név',
  displayFlagName: 'Zászló + név',
  displayFlagCode: 'Zászló + kód',
```

- [ ] **Step 3: Verify type-check (ensures en/hu key parity)**

Run: `cd web && npx tsc -b`
Expected: no errors. (`hu` is typed `Record<StringKey, string>`, so a missing key fails the build.)

- [ ] **Step 4: Commit**

```bash
git add web/src/i18n/strings.ts
git commit -m "feat(web): i18n strings for the display-mode selector"
```

---

## Task 5: Flag + TeamLabel components and CSS

**Files:**
- Create: `web/src/components/TeamLabel.tsx`
- Modify: `web/src/index.css`

- [ ] **Step 1: Create the components**

Create `web/src/components/TeamLabel.tsx`:

```tsx
import type { Team, TeamSlot } from '../graphql/types'
import { teamLabelParts } from '../lib/displayMode'
import { useResolvedDisplayMode } from '../display/useResolvedDisplayMode'

/** An 8-bit flag image. The ISO code drives the bundled asset path. */
export function Flag({ iso, name }: { iso: string; name: string }) {
  return (
    <img
      className="team-flag"
      src={`/flags/${iso}.png`}
      alt={name}
      loading="lazy"
      width={20}
      height={15}
    />
  )
}

/**
 * Render a team slot per the current display mode (flag / code / name / combo).
 * Falls back to text when a flag asset is unavailable, and shows the
 * placeholder description for unresolved knockout slots.
 */
export function TeamLabel({
  slot,
  teams,
}: {
  slot: TeamSlot
  teams: Map<string, Team>
}) {
  const mode = useResolvedDisplayMode()
  const { flag, text } = teamLabelParts(slot, teams, mode)
  return (
    <span className="team-label">
      {flag && <Flag iso={flag.iso} name={flag.name} />}
      {text && <span className="team-label-text">{text}</span>}
    </span>
  )
}
```

- [ ] **Step 2: Add the styles**

Append to `web/src/index.css`:

```css
/* 8-bit country flags — tiny PNGs scaled up with nearest-neighbour, so they
   read as chunky pixel blocks alongside the scoreboard-LED type. */
.team-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.team-flag {
  height: 14px;
  width: auto;
  image-rendering: pixelated;
  display: inline-block;
  vertical-align: middle;
  border: 1px solid var(--bg-card-border);
}

/* Header: keep the two selectors side by side. */
.header-controls {
  display: flex;
  align-items: center;
  gap: 16px;
}
```

- [ ] **Step 3: Verify type-check**

Run: `cd web && npx tsc -b`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add web/src/components/TeamLabel.tsx web/src/index.css
git commit -m "feat(web): 8-bit Flag + TeamLabel components and pixelated styling"
```

---

## Task 6: DisplayModeSelector in the header

**Files:**
- Create: `web/src/components/DisplayModeSelector.tsx`
- Modify: `web/src/components/Layout.tsx`

- [ ] **Step 1: Create the selector**

Create `web/src/components/DisplayModeSelector.tsx` (reuses the `.lang-selector` styling for consistency):

```tsx
import { useI18n } from '../i18n/useI18n'
import { useDisplayMode } from '../display/useDisplayMode'
import { DISPLAY_MODES, type DisplayMode } from '../lib/displayMode'
import type { StringKey } from '../i18n/strings'

const LABEL_KEY: Record<DisplayMode, StringKey> = {
  auto: 'displayAuto',
  flag: 'displayFlag',
  code: 'displayCode',
  name: 'displayName',
  'flag-name': 'displayFlagName',
  'flag-code': 'displayFlagCode',
}

export function DisplayModeSelector() {
  const { t } = useI18n()
  const { mode, setMode } = useDisplayMode()
  return (
    <label className="lang-selector">
      {t('display')}:{' '}
      <select
        value={mode}
        onChange={(e) => setMode(e.target.value as DisplayMode)}
      >
        {DISPLAY_MODES.map((m) => (
          <option key={m} value={m}>
            {t(LABEL_KEY[m])}
          </option>
        ))}
      </select>
    </label>
  )
}
```

- [ ] **Step 2: Add it to the header**

In `web/src/components/Layout.tsx`, add the import next to the `LanguageSelector` import:

```tsx
import { DisplayModeSelector } from './DisplayModeSelector'
import { LanguageSelector } from './LanguageSelector'
```

Then replace the bare selector in the header — change:

```tsx
        <LanguageSelector />
      </header>
```

to:

```tsx
        <div className="header-controls">
          <DisplayModeSelector />
          <LanguageSelector />
        </div>
      </header>
```

- [ ] **Step 3: Verify type-check**

Run: `cd web && npx tsc -b`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add web/src/components/DisplayModeSelector.tsx web/src/components/Layout.tsx
git commit -m "feat(web): display-mode selector in the app header"
```

---

## Task 7: Roll out `<TeamLabel>` across the render sites

Each step replaces an inline `slotLabel`/`slotCode`/`teamName` string call with `<TeamLabel>`. Do them one at a time; `tsc -b` after each catches a missed import.

**Files:** `web/src/pages/SchedulePage.tsx`, `web/src/pages/TodayPage.tsx`, `web/src/pages/AllTipsPage.tsx`, `web/src/pages/mytips/StandingsTables.tsx`, `web/src/pages/PerfectPage.tsx`, `web/src/pages/admin/AdminResults.tsx`, `web/src/pages/admin/AdminTeams.tsx`

- [ ] **Step 1: SchedulePage**

In `web/src/pages/SchedulePage.tsx`, change the import (drop `slotLabel`):

```tsx
import { byKickoff, formatKickoff, teamIndex } from '../lib/format'
import { TeamLabel } from '../components/TeamLabel'
```

Replace the match cell (currently lines ~83-84):

```tsx
                        {slotLabel(m.home, teams)} –{' '}
                        {slotLabel(m.away, teams)}
```

with:

```tsx
                        <TeamLabel slot={m.home} teams={teams} /> –{' '}
                        <TeamLabel slot={m.away} teams={teams} />
```

- [ ] **Step 2: TodayPage**

In `web/src/pages/TodayPage.tsx`, change the import (drop `slotLabel`):

```tsx
import { byKickoff, formatKickoff, teamIndex } from '../lib/format'
import { TeamLabel } from '../components/TeamLabel'
```

Replace the match cell (currently line ~117):

```tsx
                    {slotLabel(m.home, teams)} – {slotLabel(m.away, teams)}
```

with:

```tsx
                    <TeamLabel slot={m.home} teams={teams} /> –{' '}
                    <TeamLabel slot={m.away} teams={teams} />
```

- [ ] **Step 3: AllTipsPage (dense grid headers)**

In `web/src/pages/AllTipsPage.tsx`, change the import (drop `slotCode`):

```tsx
import { byKickoff, teamIndex } from '../lib/format'
import { TeamLabel } from '../components/TeamLabel'
```

Replace the grid header cell (currently line ~128):

```tsx
                    {slotCode(g.home, teams)}–{slotCode(g.away, teams)}
```

with:

```tsx
                    <TeamLabel slot={g.home} teams={teams} />–
                    <TeamLabel slot={g.away} teams={teams} />
```

(The grid follows the global mode; with `auto`+desktop it shows flag+name, which is wide — the user can switch to `flag` or `code` for a tight grid. This is the intended trade-off, not a bug.)

- [ ] **Step 4: StandingsTables (both tables)**

In `web/src/pages/mytips/StandingsTables.tsx`, replace the import block and the `teamName` helper. Change:

```tsx
import { useI18n } from '../../i18n/useI18n'
import type { Team } from '../../graphql/types'
import { type TeamStats, goalDiff } from '../../lib/standings'

function teamName(teamId: string, teams: Map<string, Team>): string {
  return teams.get(teamId)?.name ?? teamId
}
```

to:

```tsx
import { useI18n } from '../../i18n/useI18n'
import type { Team } from '../../graphql/types'
import { type TeamStats, goalDiff } from '../../lib/standings'
import { TeamLabel } from '../../components/TeamLabel'
```

Then replace **both** occurrences of the team cell (lines ~36 and ~92 — they are identical):

```tsx
              <td>{teamName(s.teamId, teams)}</td>
```

with:

```tsx
              <td>
                <TeamLabel
                  slot={{ teamId: s.teamId, description: s.teamId }}
                  teams={teams}
                />
              </td>
```

(The `description: s.teamId` preserves the old `?? teamId` fallback for an unknown id.)

- [ ] **Step 5: PerfectPage (ReactNode match labels)**

In `web/src/pages/PerfectPage.tsx`, change the import (drop `slotLabel`) and add `ReactNode` + `TeamLabel`:

```tsx
import { useMemo, type ReactNode } from 'react'
```

```tsx
import { teamIndex } from '../lib/format'
import { TeamLabel } from '../components/TeamLabel'
```

Replace the `gameLabel` builder (currently lines ~24-32):

```tsx
  const gameLabel = useMemo(() => {
    const map = new Map<string, string>()
    for (const g of tournament?.games ?? []) {
      map.set(g.id, `${slotLabel(g.home, teams)} – ${slotLabel(g.away, teams)}`)
    }
    return map
  }, [tournament, teams])
```

with:

```tsx
  const gameLabel = useMemo(() => {
    const map = new Map<string, ReactNode>()
    for (const g of tournament?.games ?? []) {
      map.set(
        g.id,
        <>
          <TeamLabel slot={g.home} teams={teams} /> –{' '}
          <TeamLabel slot={g.away} teams={teams} />
        </>,
      )
    }
    return map
  }, [tournament, teams])
```

(The consuming cell `{gameLabel.get(p.gameId) ?? p.gameId}` already accepts a `ReactNode` — no change needed there.)

- [ ] **Step 6: AdminResults (ReactNode row label)**

In `web/src/pages/admin/AdminResults.tsx`, change the import (drop `slotLabel`) and add `TeamLabel`:

```tsx
import { byKickoff, formatKickoff, teamIndex } from '../../lib/format'
import { TeamLabel } from '../../components/TeamLabel'
```

Add a `ReactNode` type import at the top of the existing React import. Find the existing `import ... from 'react'` line and ensure it includes `type ReactNode` (e.g. `import { useState, type ReactNode } from 'react'`).

Replace the `label={...}` prop (currently lines ~139-142):

```tsx
                label={`${slotLabel(game.home, teams)} – ${slotLabel(
                  game.away,
                  teams,
                )}`}
```

with:

```tsx
                label={
                  <>
                    <TeamLabel slot={game.home} teams={teams} /> –{' '}
                    <TeamLabel slot={game.away} teams={teams} />
                  </>
                }
```

Then widen the `ResultRow` prop type — change:

```tsx
  label: string
```

to:

```tsx
  label: ReactNode
```

(`ResultRow` renders `<td>{label}</td>`, which already accepts a `ReactNode`.)

- [ ] **Step 7: AdminTeams (flag column — MUST change)**

`AdminTeams` currently renders `team.flag` as emoji text; now `flag` holds an ISO code, so the old code would print "mx". Replace it with a real flag.

In `web/src/pages/admin/AdminTeams.tsx`, add imports:

```tsx
import { teamIndex } from '../../lib/format'
import { TeamLabel } from '../../components/TeamLabel'
```

After `const teams = result.data?.tournament?.teams ?? []`, add an index:

```tsx
  const teams = result.data?.tournament?.teams ?? []
  const teamMap = teamIndex(teams)
```

Replace the name cell — change:

```tsx
                <td>
                  {team.flag ? `${team.flag} ` : ''}
                  {team.name}
                </td>
```

to:

```tsx
                <td>
                  <TeamLabel
                    slot={{ teamId: team.id, description: team.name }}
                    teams={teamMap}
                  />
                </td>
```

- [ ] **Step 8: Verify type-check and lint**

Run: `cd web && npx tsc -b && npm run lint`
Expected: no type errors, no lint errors. (If lint flags an unused `slotLabel`/`slotCode`/`teamName`, remove the leftover.)

- [ ] **Step 9: Run unit tests (nothing regressed)**

Run: `cd web && npm test`
Expected: all suites PASS (existing `format.test.ts` still green — `slotLabel`/`slotCode` remain exported and used for string contexts/aria).

- [ ] **Step 10: Commit**

```bash
git add web/src/pages
git commit -m "feat(web): render team flags via TeamLabel across all views"
```

---

## Task 8: End-to-end test for the toggle

**Files:**
- Create: `web/e2e/team-display.spec.ts`

The e2e harness boots the whole live stack (`e2e/global-setup.ts`), imports `fwc26.json` (now carrying ISO codes), and serves `web/public/flags/`. This test uses the public `/games` schedule so it needs no login.

- [ ] **Step 1: Ensure dev-stub auth env exists (so auth-gated UI isn't hidden)**

If `web/.env.local` does not exist, create it to blank the Auth0 vars (keeps the app in dev-stub mode for e2e):

```bash
test -f web/.env.local || printf 'VITE_AUTH0_DOMAIN=\nVITE_AUTH0_CLIENT_ID=\n' > web/.env.local
```

- [ ] **Step 2: Write the e2e spec**

Create `web/e2e/team-display.spec.ts`:

```ts
import { test, expect } from '@playwright/test'
import { expectNoErrorView, watchNetwork } from './helpers'

/**
 * Team display-mode toggle. The selector is a global preference (localStorage);
 * `/games` is public, so no login is needed. We assert real flag PNGs load
 * (naturalWidth > 0 — a 404 would be 0), that name text shows/hides per mode,
 * and that the choice survives a reload.
 */
test('display-mode selector switches flags/names and persists', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.goto('/games')
  await expect(page.locator('h2')).toHaveText('Schedule')
  await expectNoErrorView(page)

  const selector = page.locator('.header-controls select').first()

  // Flag + name: both a flag image and the country name are present.
  await selector.selectOption('flag-name')
  const firstFlag = page.locator('img.team-flag').first()
  await expect(firstFlag).toBeVisible()
  // The bundled PNG actually resolves (not a broken 404 image).
  await expect
    .poll(async () => firstFlag.evaluate((img: HTMLImageElement) => img.naturalWidth))
    .toBeGreaterThan(0)
  await expect(page.locator('.team-label-text').first()).toBeVisible()

  // Flag only: images remain, text labels disappear.
  await selector.selectOption('flag')
  await expect(page.locator('img.team-flag').first()).toBeVisible()
  await expect(page.locator('.team-label-text')).toHaveCount(0)

  // Name only: text returns, flags disappear.
  await selector.selectOption('name')
  await expect(page.locator('.team-label-text').first()).toBeVisible()
  await expect(page.locator('img.team-flag')).toHaveCount(0)

  // Persists across reload.
  await page.reload()
  await expect(page.locator('h2')).toHaveText('Schedule')
  await expect(page.locator('img.team-flag')).toHaveCount(0)
  await expect(page.locator('.team-label-text').first()).toBeVisible()
  await expect(selector).toHaveValue('name')

  net.assertNoPageErrors()
  await net.assertNoGraphqlErrors()
})
```

- [ ] **Step 3: Run the e2e test**

Run: `cd web && npm run e2e -- team-display`
Expected: PASS. (First run boots docker/DynamoDB/API — may take a minute.)

- [ ] **Step 4: Commit**

```bash
git add web/e2e/team-display.spec.ts web/.env.local
git commit -m "test(web): e2e for the team display-mode toggle"
```

(If `web/.env.local` is git-ignored, the `git add` simply adds nothing for it — that's fine.)

---

## Task 9: Document the `flag`-field repurposing

**Files:**
- Modify: `.specs/DATA_MODEL.md`

- [ ] **Step 1: Add a note in the data-model spec**

Open `.specs/DATA_MODEL.md`, find the `Team` definition / field description, and add a note (place it in the corrections section if one exists, otherwise near the `Team` fields):

```markdown
> **`Team.flag` holds an ISO-3166-1 alpha-2 country code** (lowercase, e.g.
> `mx`, `br`; GB subdivisions `gb-eng` / `gb-sct` for England / Scotland), not a
> flag emoji. The SPA derives the bundled 8-bit flag asset path from it
> (`/flags/<code>.png`). Placeholder teams with no fixed country keep `flag: null`
> and degrade to text. See
> `docs/superpowers/specs/2026-06-06-country-flags-display-mode-design.md`.
```

- [ ] **Step 2: Commit**

```bash
git add .specs/DATA_MODEL.md
git commit -m "docs(specs): note Team.flag holds an ISO country code"
```

---

## Final verification

- [ ] **Run the whole frontend gate:**

```bash
cd web && npx tsc -b && npm run lint && npm test && npm run build
```
Expected: type-check clean, lint clean, unit tests green, production build succeeds.

- [ ] **Run the full e2e suite** (catches any regression in the rolled-out pages):

```bash
cd web && npm run e2e
```
Expected: all specs PASS.

- [ ] **Manual smoke (optional):** `bin/tmux country-flags`, open `:5173`, cycle the **Show** selector through all six modes on `/games`, `/today`, `/alltips`, `/scoreboard`/standings, and `/admin/teams`; confirm flags render as chunky pixel art and the choice sticks across reload.

---

## Self-review notes (coverage of the spec)

- **§1 data / ISO in `flag`** → Task 1 (data), Task 9 (doc).
- **§2 tiny-PNG + pixelated, bundled, fetch script** → Task 1 (script + assets), Task 5 (`.team-flag { image-rendering: pixelated }`).
- **§3 mode model, context, localStorage, matchMedia, selector** → Task 2 (resolver), Task 3 (context/hooks), Task 4 (strings), Task 6 (selector).
- **§4 `<TeamLabel>`, fallbacks, all 7 render sites, CSS** → Task 2 (`teamLabelParts` fallbacks), Task 5 (components + CSS), Task 7 (rollout).
- **§5 testing: unit / e2e** → Task 2 (unit), Task 8 (e2e). (Component rendering is covered by e2e, matching the repo's node-env Vitest scope which targets `src/lib/` pure logic only.)
