# Country flags + display-mode toggle — design

**Date:** 2026-06-06
**Status:** Approved (design); ready for implementation plan
**Branch / worktree:** `feat/country-flags` → `.claude/worktrees/country-flags`

## Goal

Show an 8-bit, retro-styled flag for each country, and let the user switch how
teams are displayed across the app: flag only, 3-letter code only, full name
only, flag + name, or flag + code. The default is responsive — flag-only on
mobile, flag + name on larger screens.

The flags are the **first raster image assets** in the SPA (everything today is
SVG or CSS), and must fit the existing scoreboard-LED / CRT aesthetic.

## Decisions (from brainstorming)

- **Flag rendering:** flag *images*, deliberately 8-bit/retro styled.
- **Technique:** tiny PNGs rendered scaled-up with `image-rendering: pixelated`
  — nearest-neighbour upscaling produces authentic chunky pixel blocks. Assets
  are **bundled** into the repo (no runtime network dependency; keeps offline
  dev and e2e deterministic).
- **Toggle:** a single app-wide preference, selector in the header next to the
  language selector — mirrors the i18n pattern.
- **Default:** responsive `auto` — flag-only on mobile, flag + name on bigger
  screens.
- **Modes offered:** flag only, 3-letter code only, full name only, flag + name,
  flag + code (plus `auto`).

## 1. Data — where flags come from

The domain `Team` already has `flag: Option<String>` (in `crates/domain/src/model.rs`),
which flows unchanged through storage (`crates/storage`) and GraphQL
(`crates/api/src/gql/types.rs`, exposed as `flag` on the `Team` type). It is
currently `null` for every team in `tournaments/fwc26.json`.

**Decision: repurpose `flag` to hold the ISO-3166-1 alpha-2 country code**
(lowercase, e.g. `"mx"`, `"br"`, `"us"`). The SPA derives the asset path from
it: `flag: "mx"` → `/flags/mx.png`.

- **No Rust changes.** The field already exists end-to-end, so we avoid touching
  `model.rs` (the locked domain contract) and the storage/GraphQL/import
  plumbing. We only populate values in `tournaments/fwc26.json`.
- **Trade-off:** a field literally named `flag` holding an ISO code is mildly
  surprising for a future reader. This is documented here and should be noted in
  `.specs/DATA_MODEL.md`. The rejected alternative — adding a dedicated
  `country_code` field — ripples through the locked domain model, storage
  serialization, GraphQL types, and the importer, for no functional gain (YAGNI).
- **Placeholder teams** (intercontinental / UEFA playoff slots with no fixed
  country) keep `flag: null` and degrade gracefully (see §4 fallbacks).

The ISO code is the single source of truth for both the flag asset and the
asset-fetch script (§2). It lives in the tournament data, not a hardcoded
frontend mapping table.

## 2. Assets — tiny PNGs + pixelated CSS

- Vendor ~20px-wide flag PNGs into `web/public/flags/<iso2>.png`. Served as-is
  by Vite (same place as `favicon.svg` / `icons.svg`).
- Rendered scaled-up (target render height aligned to the LED label baseline,
  e.g. ~16–20px tall) with `image-rendering: pixelated` for the chunky 8-bit
  look.
- A reproducible fetch script — `bin/fetch-flags` (or an npm script under
  `web/`) — downloads the PNGs from **flagcdn.com** (public-domain flags, based
  on flagpedia; free for any use) for exactly the ISO codes present in
  `tournaments/fwc26.json`. The downloaded PNGs are committed so dev and e2e
  never hit the network.
- Script behaviour: read `fwc26.json`, collect non-null `flag` ISO codes,
  download each `https://flagcdn.com/20x15/<iso2>.png` (or the closest small
  size) to `web/public/flags/<iso2>.png`, skip ones already present.

## 3. Display-mode model & toggle

- **Type:** `DisplayMode = 'auto' | 'flag' | 'code' | 'name' | 'flag-name' | 'flag-code'`.
- **Context + persistence:** a `DisplayModeProvider` (sibling to `I18nProvider`)
  holds the mode in React state, persisted to `localStorage` under
  `xpool.displayMode`, defaulting to `'auto'`. A `useDisplayMode()` hook exposes
  `{ mode, setMode }`. This is a near-copy of the i18n provider pattern
  (`web/src/i18n/I18nContext.tsx`).
- **Resolution:** a `useResolvedDisplayMode()` hook returns one of the five
  *concrete* renderings (`flag | code | name | flag-name | flag-code`). For
  `'auto'` it reads a `matchMedia('(max-width: 640px)')` hook → `flag` on mobile,
  `flag-name` otherwise. The match-media listener lives in one place and updates
  live on resize/rotate.
- **Resolution logic is a pure function** `resolveDisplayMode(mode, isMobile)`
  so it can be unit-tested without a DOM; the hook only supplies `isMobile`.
- **Selector UI:** a `DisplayModeSelector` `<select>` in the app header next to
  `LanguageSelector` (`web/src/components/`). Options localized via new i18n
  string keys (`displayAuto`, `displayFlag`, `displayCode`, `displayName`,
  `displayFlagName`, `displayFlagCode`), styled with the existing
  `Share Tech Mono` selector styling.

## 4. `<TeamLabel>` component & rollout

Today every site renders a *string* via `slotLabel()` / `slotCode()` (in
`web/src/lib/format.ts`). Flags are images, so a string helper can't express
them. Introduce a React component:

```tsx
<TeamLabel slot={game.home} teams={teams} />
```

Driven by `useResolvedDisplayMode()`:

- **Resolved team** → renders per the concrete mode: a `<Flag>` image, the
  `shortCode`, the `name`, or flag + text.
- **Flag wanted but `flag` is null** (placeholder teams) → fall back to code (or
  name in name-ish modes) so nothing renders empty.
- **Unresolved slot** (no `teamId`) → render `slot.description`
  ("Winner SF 1", "2A") as text, in every mode, no flag.

A leaf `<Flag iso={} name={} />` component owns the `<img class="team-flag">`
(with `alt={name}`, `loading="lazy"`) and the pixelated rendering.

`slotLabel` / `slotCode` are **kept** for the cases that still need a plain
string — accessible labels, `title`/`aria` attributes, and sort keys.

**Rollout — replace the inline string usages at the 7 render sites** found in
exploration:

| Site | File |
|---|---|
| Schedule | `web/src/pages/SchedulePage.tsx` |
| Today / Fresh | `web/src/pages/TodayPage.tsx` |
| All Tips (dense grid headers) | `web/src/pages/AllTipsPage.tsx` |
| Standings (predicted + actual) | `web/src/pages/mytips/StandingsTables.tsx` |
| Perfect predictions | `web/src/pages/PerfectPage.tsx` |
| Admin results entry | `web/src/pages/admin/AdminResults.tsx` |
| Admin teams list | `web/src/pages/admin/AdminTeams.tsx` |

Match rows like `home – away` become `<TeamLabel/> – <TeamLabel/>`.

**CSS** (`web/src/index.css`): add `.team-flag { image-rendering: pixelated;
height: <baseline>; width: auto; }` and a `.team-label` flex wrapper aligning
flag + text on the Press Start 2P / VT323 baselines.

## 5. Testing

Per project workflow (frontend work needs an E2E) and the repo's clock/e2e
conventions (`.specs/TESTING.md`):

- **Unit:** `resolveDisplayMode(mode, isMobile)` — `auto`+mobile → `flag`,
  `auto`+desktop → `flag-name`, and each explicit mode passes through unchanged.
- **Component:** `<TeamLabel>` rendering matrix — a resolved team in each
  concrete mode; the null-flag fallback; the unresolved-slot description path.
- **E2E (Playwright):** on a page with matches, switch the selector to
  flag-only / name-only / both; assert the flag `<img>` appears and resolves
  (`naturalWidth > 0`), the name shows/hides accordingly, and the choice
  persists across reload (localStorage). Reuses the existing live-stack e2e
  harness; requires the `web/.env.local` dev-stub-auth blanking so the
  `.auth-bar` (and selectors) are visible.

## Out of scope (YAGNI)

- Per-page / per-component display modes (one global preference only).
- A dedicated settings page (selector lives inline in the header).
- Hand-crafted pixel-art flags or a flag SVG library (tiny-PNG + pixelated CSS
  achieves the look with full 48-team coverage).
- Adding a `country_code` domain field (reuse the existing `flag` field).
