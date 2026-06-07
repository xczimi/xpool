# Theme switcher — accent colours + dark/light mode

Status: approved (brainstorming)
Area: web
Source idea: `.scratch/theme-switcher/PRD.md`
Branch/worktree: `worktree-theme-switcher` (`.claude/worktrees/theme-switcher`)

## Goal

Let users pick the app's accent colour from a small set of presets (replacing the
single fixed orange) and choose a Dark / Light / System mode, controlled from the
chrome next to the language picker. Persisted per-device so it survives reloads.

## Owner decisions (locked during brainstorming)

1. **Six accent presets**, LED-native (amber default + green, cyan, magenta,
   violet, mono).
2. **Dark is the primary aesthetic; Light is a lightweight-but-real inverted
   palette** this cut (not a fully tuned light design — refined later).
3. **Persistence: localStorage now, profile sync later** (clean seam, no schema
   change this cut).
4. **Mode is 3-way System / Dark / Light**; System follows
   `prefers-color-scheme`. Full plumbing built now.
5. **Controls: segmented / swatch** (Approach A), not the existing `<select>`
   convention — this also sets the template the language-picker-redesign idea
   (`.scratch/language-picker-redesign/`) will follow, since they share this chrome.

## Architecture (Approach A)

Two **orthogonal axes** expressed as attributes on `document.documentElement`:

- `data-accent ∈ {amber, green, cyan, magenta, violet, mono}` — the accent hue.
- `data-theme ∈ {dark, light}` — the *resolved* mode (System is resolved away
  before it reaches the DOM).

CSS attribute-selector blocks in `index.css` override token values; **components
are untouched**. A `ThemeProvider` (twin of `DisplayModeProvider`) holds the two
preference values, persists them to localStorage, applies the attributes via an
effect, and subscribes to `matchMedia('(prefers-color-scheme: dark)')` while the
mode is `system`.

```
ThemeProvider (state + localStorage + matchMedia + DOM attributes)
  └─ useTheme() ──> ThemeSelector (swatches + System/Dark/Light segmented control)
index.css: :root baseline + [data-accent="…"] + [data-theme="light"] overrides
```

### 1. Token architecture (`index.css`)

The accent is currently the amber spectrum (`--amber` ×20, `--amber-bright` ×16,
`--amber-dim` ×11 = 47 uses) plus the `--accent` alias. Surfaces are `--bg-*`,
text is `--text-*`.

- Introduce **semantic accent tokens** `--accent`, `--accent-bright`,
  `--accent-dim` as the source of truth, and **alias the existing names to them**:
  `--amber: var(--accent)`, `--amber-bright: var(--accent-bright)`,
  `--amber-dim: var(--accent-dim)`. All 47 `var(--amber*)` usages keep working
  with **zero component churn**. (This inverts today's `--accent: var(--amber)`
  alias direction.)
- The current `:root` values become the **amber × dark** baseline.
- Per-accent blocks `[data-accent="green"] { --accent: …; --accent-bright: …;
  --accent-dim: … }` set only the three accent vars.
- The light-mode block `[data-theme="light"] { … }` swaps `--bg-*` / `--text-*`
  surfaces and softens the CRT scanline.
- **Tokenise the two stray hardcoded hex** (the header gradient at `index.css:93`
  and the `#0a0a14` at `index.css:152`) so light mode is complete rather than
  leaking dark surfaces.

Accent and mode are independent: any of 6 accents combines with dark or light.

### 2. Six accents

LED-native palette. Exact hexes are tuned during the visual/E2E check; these are
the starting values:

| key                | `--accent` | `--accent-bright` | `--accent-dim` |
|--------------------|------------|-------------------|----------------|
| `amber` (default)  | `#ff8c00`  | `#ffd76a`         | `#a04400`      |
| `green`            | `#33ff66`  | `#8effb0`         | `#14a33d`      |
| `cyan`             | `#21d4fd`  | `#8be9ff`         | `#0a7fa0`      |
| `magenta`          | `#ff49d0`  | `#ffa6ef`         | `#9c1f86`      |
| `violet`           | `#9d6bff`  | `#c9b0ff`         | `#5a3aa0`      |
| `mono`             | `#e8e8f0`  | `#ffffff`         | `#8a8a99`      |

`amber` is the baseline already in `:root`; it needs no `[data-accent]` override
(or gets an explicit one for symmetry — implementer's choice).

### 3. Mode: System / Dark / Light

- `themeMode ∈ {system, dark, light}` is what the user selects.
- The provider resolves it to an effective `dark | light`:
  `system → prefers-color-scheme` (defaulting to `dark` when unknown);
  `dark`/`light` pass through. The resolved value is written to `data-theme`.
- **Light** is a lightweight inverted palette: warm-paper surfaces, dark ink,
  scanline dialled down. Honest and usable; full polish deferred.
- While mode is `system`, the provider listens for `matchMedia` change events and
  re-resolves live. When the user picks an explicit mode, the listener is removed.

### 4. File layout (mirrors `web/src/display/`)

- `web/src/theme/theme.ts` — **pure, unit-tested**: `ACCENTS` (ordered list),
  `THEME_MODES`, `Accent` / `ThemeMode` / `ResolvedTheme` types,
  `resolveTheme(mode, systemPrefersDark)`, storage-key constants
  (`xpool.accent`, `xpool.themeMode`), and validation helpers
  (`coerceAccent` / `coerceThemeMode` that fall back to defaults on junk).
- `web/src/theme/themeContextValue.ts` — `ThemeContext` + `ThemeState` interface
  (`accent`, `mode`, `resolved`, `setAccent`, `setMode`).
- `web/src/theme/ThemeProvider.tsx` — state, localStorage read/write (try/catch,
  same shape as `DisplayModeProvider`), DOM-attribute effect, `matchMedia` wiring.
- `web/src/theme/useTheme.ts` — context hook (throws outside provider).
- `web/src/components/ThemeSelector.tsx` — accent swatches (a `radiogroup`) +
  System/Dark/Light segmented control. Accessible: keyboard navigable,
  `aria-label`/`aria-checked`, i18n strings.
- `web/src/i18n/strings.ts` — new keys (en + hu): the control labels, the six
  accent names, and the three mode names.
- `web/src/components/Layout.tsx` — mount `<ThemeSelector />` in
  `.header-controls`, next to `<LanguageSelector />`.
- `web/src/main.tsx` — wrap the app in `<ThemeProvider>` (alongside the existing
  providers).

### 5. Persistence — localStorage now, profile seam later

- Keys `xpool.accent` and `xpool.themeMode`, written through the same try/catch
  helper shape as `xpool.displayMode` / `xpool.locale`.
- No GraphQL / schema / storage change this cut.
- **Seam:** the pure `theme.ts` (defaults + validation) and the provider's
  read/write boundary are where a future per-account sync slots in — it swaps the
  storage backend without touching `theme.ts` or `ThemeSelector`.

### 6. Initial-paint note

To avoid a one-frame flash of the default accent before React hydrates, the
provider reads localStorage and applies the attributes in a layout effect on
mount. A tiny inline pre-hydration script in `index.html` is **optional** and out
of scope for this cut; if flash is observable in the E2E run, add it then.

## Testing (TDD)

**Unit — `web/src/theme/theme.test.ts` (node env, written first):**
- `coerceAccent` / `coerceThemeMode` reject unknown/empty values and return the
  defaults (`amber`, `system`); accept every valid value.
- `resolveTheme('system', true) === 'dark'`, `resolveTheme('system', false) ===
  'light'`; `resolveTheme('dark', _) === 'dark'`; `resolveTheme('light', _) ===
  'light'`.
- `ACCENTS` contains exactly the six keys, in order, `amber` first.

The provider / DOM wiring is verified by E2E (matching the project pattern where
`lib/displayMode.ts` is unit-tested but `DisplayModeProvider` is not).

**E2E — `web/e2e/theme.spec.ts` (Playwright, visitor / no auth):**
- Default load: `<html data-accent="amber" data-theme="dark">`.
- Pick a non-amber accent → `data-accent` updates **and** a computed accent
  colour on a header element changes accordingly.
- Toggle Dark → Light → `data-theme` flips to `light` and a surface colour
  changes.
- **Reload persists** both choices (localStorage).
- System mode honours `prefers-color-scheme` via Playwright's `colorScheme`
  emulation (`page.emulateMedia({ colorScheme: 'light' })` ⇒ resolved `light`).

Theme switching is available to logged-out visitors, so this spec needs **no
dev-stub auth** (no `web/.env.local` blanking required).

## Out of scope (seams left)

- Language-picker `<select>` → segmented redesign (its own idea; this cut just
  sits beside it and sets the visual template).
- Per-account profile sync (localStorage backend is swappable later).
- Fully colour-graded / contrast-audited light theme (lightweight this cut).
- Pre-hydration inline anti-flash script (add only if flash is observed).

## Verification checklist

- [ ] `npm test` (vitest) green — `theme.test.ts` passes.
- [ ] `npm run build` (`tsc -b && vite build`) clean.
- [ ] `npm run lint` clean.
- [ ] `npm run e2e -- theme.spec.ts` green.
- [ ] Manual: all 6 accents × dark/light render legibly; no hardcoded-hex leaks.
