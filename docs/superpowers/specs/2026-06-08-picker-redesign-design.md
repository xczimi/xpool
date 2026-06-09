# Picker redesign — segmented toggles for language & display

Date: 2026-06-08
Branch: `picker-redesign`
Area: web (header chrome)
Supersedes: `.scratch/language-picker-redesign/PRD.md`

## Iteration (post-review): collapse behind a gear + label every row

After building the toggles inline in the header, two problems surfaced on
review: (1) the four controls took too much horizontal space by default, and
(2) an unlabelled toggle gives no hint of what it controls — the `aria-label`
is invisible.

Both are fixed by a **settings gear**: the chrome preferences collapse into a
popover opened from a single gear button in the header, and every control sits
in a **labelled row** (visible text label + its toggle). All four preferences
move into the panel — language, display flag, display text, theme accent, theme
mode.

- `SegToggle` / `SegRow` (`components/SegToggle.tsx`) — a reusable labelled
  segmented toggle (and its bare label-row wrapper, for the accent swatches).
  The language, display and theme-mode pickers all render through it, so a row
  is "visible label + `.seg-toggle`" everywhere.
- `SettingsMenu` (`components/SettingsMenu.tsx`) — the gear button
  (`aria-haspopup="dialog"` / `aria-expanded`) and the popover (`role="dialog"`,
  labelled `Settings`). Closes on Escape or an outside `mousedown`. Hosts the
  `LanguageSelector`, `DisplayModeSelector` and `ThemeSelector`.
- `Layout` renders `<SettingsMenu/>` in `.header-controls` in place of the three
  inline selectors.
- New i18n key `settings`. The visible row labels reuse existing keys
  (`language`, `displayFlag`, `displayText`, `theme`, `mode`).
- E2E: a new `settings-menu.spec.ts` (gear hidden-by-default → reveals a
  labelled panel → Escape / outside-click close); the existing display / i18n /
  theme specs open the gear first via an `openSettings` helper.

The rest of this document describes the underlying two-axis display model, which
is unchanged by the iteration.

## Problem

Two header controls are still `<select>` dropdowns sharing the `.lang-selector`
class:

- **`LanguageSelector`** — a binary EN/HU choice hidden behind a dropdown.
- **`DisplayModeSelector`** (the "Display:" / "show" picker) — a single 6-value
  enum (`auto | flag | code | name | flag-name | flag-code`) crammed into one
  `<select>`, mixing two orthogonal concerns (whether to show a flag, and what
  text to show).

Meanwhile the recently shipped **`ThemeSelector`** established the house style: a
segmented `role="radiogroup"` of buttons (`System | Dark | Light`). The two
dropdowns now look out of place next to it.

## Goal

Replace both dropdowns with segmented toggles in the `ThemeSelector` style, so
the four header controls (display-flag, display-text, language, theme) read as
one family. Split the conflated display enum into two independent axes.

## The two display pickers

The single `Display:` dropdown becomes **two** segmented toggles.

### Flag — `On | Off`

A plain boolean. Flags have no responsive variance today (a flag is either shown
or not), so this axis is two segments, not three. No `Auto`.

### Text — `Auto | Name | Code | Off`

- `Auto` — responsive. **Name** on desktop. On a narrow phone: **none** when a
  flag is shown (compact flag-only), otherwise the short **Code** — so the
  label is never empty even with Flag `Off`. This is flag-aware, which is why
  `composeDisplayMode` takes both axes.
- `Name` — always the full team name.
- `Code` — always the short code.
- `Off` — no text label.

### Default

Flag `On` + Text `Auto`. This reproduces today's `auto` mode exactly:
flag + name on desktop, flag-only on mobile.

### Mode mapping

The two axes reproduce all six of today's modes, plus a few new sensible combos:

| Flag | Text | concrete rendering (desktop)     | = today's mode |
|------|------|----------------------------------|----------------|
| On   | Auto | flag + name (mobile: flag only)  | `auto`         |
| On   | Name | flag + name                      | `flag-name`    |
| On   | Code | flag + code                      | `flag-code`    |
| On   | Off  | flag only                        | `flag`         |
| Off  | Name | name                             | `name`         |
| Off  | Code | code                             | `code`         |
| Off  | Auto | name (mobile: code)              | *(new)*        |

On mobile, the `On | Auto` cell resolves to flag-only and `Off | Auto` resolves
to code — both exactly matching today and never empty.

### The one invalid combo

Flag `Off` + Text `Off` renders nothing. Guard it in the UI: **disable the Text
`Off` segment while Flag is `Off`.** No combination that produces an empty label
is reachable. (Defense in depth: the compose function below also never returns an
empty rendering — `teamLabelParts` already falls back to the short code when a
flag asset is missing.)

## Blast radius — kept deliberately small

The rendering layer stays untouched. `TeamLabel`, `teamLabelParts`, and
`compactMode` keep consuming the existing `ConcreteDisplayMode` enum
(`flag | code | name | flag-name | flag-code`). That enum remains the internal
rendering contract; only its *source* changes — from one stored enum to a
composed `(flag, text, viewport)` value.

Changes are confined to:

1. **`crates/`** — none. This is web-only.

2. **`web/src/lib/displayMode.ts`**
   - Add `FlagMode = 'on' | 'off'` and `TextMode = 'auto' | 'name' | 'code' | 'off'`.
   - Add `composeDisplayMode(flag: FlagMode, text: TextMode, isMobile: boolean): ConcreteDisplayMode`
     — the single source of truth for the mapping table above. Replaces
     `resolveDisplayMode` (which is deleted along with the `auto` member of
     `DisplayMode`; the old `DisplayMode` type itself goes away).
   - Keep `ConcreteDisplayMode`, `teamLabelParts`, `compactMode`, `FLAG_MODES`
     exactly as-is.

3. **`web/src/display/` (provider + context + hooks)**
   - `DisplayModeProvider` stores **two** values: `flag` and `text`. Persist to
     two keys (`xpool.display.flag`, `xpool.display.text`).
   - **One-time migration**: on init, if the legacy `xpool.displayMode` key
     exists, translate its enum value into `(flag, text)` via the mapping table,
     write the new keys, and remove the legacy key.
   - Context exposes `{ flag, text, setFlag, setText }` (immutable updates).
   - `useResolvedDisplayMode` composes `(flag, text, isMobile)` →
     `ConcreteDisplayMode` via `composeDisplayMode`. Its public signature is
     unchanged, so `TeamLabel` is untouched.

4. **`web/src/components/DisplayModeSelector.tsx`**
   - Renders two segmented `radiogroup`s (Flag, Text) instead of a `<select>`.
   - The Text `off` segment is `disabled` (and `aria-disabled`) when `flag === 'off'`.

5. **`web/src/components/LanguageSelector.tsx`**
   - Renders a segmented `EN | HU` `radiogroup` instead of a `<select>`.
   - Labels from `localeNames` / i18n, two-letter uppercase codes on the
     segments (`EN`, `HU`) with full-name `aria-label`/`title`.

6. **`web/src/index.css`**
   - Generalize `.mode-toggle` / `.mode-option` into a reusable
     `.seg-toggle` / `.seg-option` (+ `.is-active`, `:disabled`) pair.
   - `ThemeSelector` keeps working — either re-point its classes to the shared
     ones, or have `.mode-toggle` extend `.seg-toggle`. Prefer renaming
     `ThemeSelector`'s markup to the shared classes so there's one toggle style.
   - Remove the now-unused `.lang-selector` rule.

## i18n

New string keys for the segment labels and group `aria-label`s, in
`web/src/i18n/strings.ts` (EN + HU):

- Flag group: label + `flagOn` / `flagOff`.
- Text group: label + `textAuto` / `textName` / `textCode` / `textOff`.
- Language group `aria-label` (reuse existing `language`).

The existing `display*` keys (`displayAuto`, `displayFlag`, …) and the
`LABEL_KEY` map in `DisplayModeSelector` are removed.

## Accessibility

Mirror `ThemeSelector`: each toggle is a `role="radiogroup"` with an
`aria-label`; each segment is a `<button role="radio" aria-checked>`. Keyboard
operable (native button focus + activation). The disabled Text-`off` segment
sets `disabled` so it leaves the tab order while Flag is `Off`.

## Testing

- **Unit (`displayMode.test.ts`)** — replace the `resolveDisplayMode` tests with
  `composeDisplayMode` tests covering all 12 `(flag, text)` × `{mobile, desktop}`
  combinations, asserting each resolves to the expected `ConcreteDisplayMode`.
- **Unit (provider)** — legacy-key migration: each of the six old enum values
  migrates to the correct `(flag, text)` pair and clears `xpool.displayMode`.
- **E2E (Playwright)** — a header-controls spec: toggling Flag/Text/Language
  updates the rendered team labels and persists across reload; the Text `Off`
  segment is disabled when Flag is `Off`. (Per house rule: frontend work is
  verified end-to-end, not just build+lint.)

## Out of scope

- The theme picker itself (already shipped) — only its CSS classes are
  generalized.
- A third language. The language toggle is built for the two locales in
  `localeNames`; it maps over them, so a third would render a third segment, but
  we are not adding one now.
- Any `crates/` / GraphQL change. Purely client-side presentation.

## Open question resolved

- *Flags vs codes vs names on the toggle?* — Resolved: the language toggle uses
  two-letter uppercase codes (`EN | HU`) with full-name `aria-label`/`title`,
  matching the mono-uppercase chrome aesthetic.
