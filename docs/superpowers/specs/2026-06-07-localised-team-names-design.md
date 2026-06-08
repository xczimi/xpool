# Localised team names (per-locale display names)

Date: 2026-06-07
Status: approved (pending user spec review)
Area: `web/` i18n only — no domain / API / storage changes
Supersedes the relevant slice of `.scratch/country-name-variants/`

## Problem

`Team.name` (`crates/domain/src/model.rs`) is a single English string sourced
from `tournaments/fwc26.json` (`Mexico`, `Czechia`, `Croatia`, …). xPool is
i18n-first (EN + HU), but team names are **not** translated: a Hungarian user
sees "Czechia" and "Croatia" instead of "Csehország" and "Horvátország". Team
names are exactly the strings a Hungarian audience will notice if they're wrong.

## Scope (deliberately narrow)

In scope: **per-locale display names only** (the "localised exonym" axis from the
backlog note). When the UI language is Hungarian, render Hungarian country names.

Explicitly **out of scope** (YAGNI — no current UI surface needs them):

- official long-form / historical names (`Hungarian People's Republic`, era drift)
- a common-vs-official **register** axis
- any change to the domain model, GraphQL schema, storage, or the imported
  tournament JSON

The English name stays in `fwc26.json` and remains the fallback. This keeps the
**locked `model.rs` contract** untouched and confines the change to `web/`.

## Approach

Resolve the localised name **at the `teamIndex` boundary** — the single place
every page turns the GraphQL team list into a `Map<id, Team>`. `teamIndex` gains
a `locale` argument and returns teams whose `name` is the localised display name
(falling back to the English JSON name). Everything downstream is untouched and
localised for free: `teamLabelParts`, `slotLabel`, flag `alt`/`title` text, and
the admin sort.

Rejected alternatives:

- **Thread a resolver into the display functions** (`teamLabelParts`,
  `slotLabel`): more call sites change and the pure display unit tests need
  rewriting, for no benefit over localising once at the index.
- **Localise server-side**: touches the locked `model.rs` contract and puts i18n
  data in the wrong layer.

## Components

### 1. `web/src/i18n/teamNames.ts` (new)

The localised-name catalogue, co-located with the rest of the i18n strings,
keyed by the stable language-neutral `shortCode`:

```ts
import type { Locale } from './strings' // Locale is declared in strings.ts

// Localised team display names, keyed by Team.shortCode. A locale with no entry
// for a team (or an omitted locale) falls back to the English JSON name, so the
// roster can change without touching this file.
export const teamNames: Partial<Record<Locale, Record<string, string>>> = {
  hu: { /* full FWC26 roster — see data table below */ },
  // `en` omitted: English names already come from fwc26.json.
}
```

### 2. Resolver — `teamDisplayName(team, locale)`

A small pure helper (co-located in `teamNames.ts` or `format.ts`):

```ts
export function teamDisplayName(team: Team, locale: Locale): string {
  return teamNames[locale]?.[team.shortCode] ?? team.name
}
```

Behaviour: catalogue hit → localised name; miss or unknown locale → English
`team.name`; never blank.

### 3. `teamIndex(teams, locale)` — localise once

`web/src/lib/format.ts`:

```ts
export function teamIndex(teams: Team[], locale: Locale): Map<string, Team> {
  return new Map(
    teams.map((t) => [t.id, { ...t, name: teamDisplayName(t, locale) }]),
  )
}
```

Immutable — a new team object per entry (coding-style: never mutate). The
overwritten `name` becomes "display name in the current locale"; downstream only
uses it for display, flag alt text, and sorting, all of which should be
localised.

### 4. Call-site updates

Each page that builds a team index passes `locale` from `useI18n()` and adds it
to the `useMemo` dependency array:

- `web/src/pages/SchedulePage.tsx`
- `web/src/pages/PerfectPage.tsx`
- `web/src/pages/AllTipsPage.tsx`
- `web/src/pages/TodayPage.tsx`
- `web/src/pages/admin/AdminTeams.tsx` (also sorts by `name` — picks up
  `localeCompare` on the localised name automatically)

`DevClock.tsx` maps team id → `shortCode` only, so it needs no change.

## Data — Hungarian roster (review before go-live)

Pre-populated for the current `fwc26.json` roster (48 teams). Wording is the
common Hungarian country name; correct any of these before go-live. Any team not
listed (or added later) falls back to its English name.

| code | English | Hungarian |
|------|---------|-----------|
| ALG | Algeria | Algéria |
| ARG | Argentina | Argentína |
| AUS | Australia | Ausztrália |
| AUT | Austria | Ausztria |
| BEL | Belgium | Belgium |
| BIH | Bosnia and Herzegovina | Bosznia-Hercegovina |
| BRA | Brazil | Brazília |
| CAN | Canada | Kanada |
| CIV | Ivory Coast | Elefántcsontpart |
| COD | DR Congo | Kongói DK |
| COL | Colombia | Kolumbia |
| CPV | Cape Verde | Zöld-foki Köztársaság |
| CRO | Croatia | Horvátország |
| CUW | Curacao | Curaçao |
| CZE | Czechia | Csehország |
| ECU | Ecuador | Ecuador |
| EGY | Egypt | Egyiptom |
| ENG | England | Anglia |
| ESP | Spain | Spanyolország |
| FRA | France | Franciaország |
| GER | Germany | Németország |
| GHA | Ghana | Ghána |
| HAI | Haiti | Haiti |
| IRN | Iran | Irán |
| IRQ | Iraq | Irak |
| JOR | Jordan | Jordánia |
| JPN | Japan | Japán |
| KOR | South Korea | Dél-Korea |
| KSA | Saudi Arabia | Szaúd-Arábia |
| MAR | Morocco | Marokkó |
| MEX | Mexico | Mexikó |
| NED | Netherlands | Hollandia |
| NOR | Norway | Norvégia |
| NZL | New Zealand | Új-Zéland |
| PAN | Panama | Panama |
| PAR | Paraguay | Paraguay |
| POR | Portugal | Portugália |
| QAT | Qatar | Katar |
| RSA | South Africa | Dél-Afrika |
| SCO | Scotland | Skócia |
| SEN | Senegal | Szenegál |
| SUI | Switzerland | Svájc |
| SWE | Sweden | Svédország |
| TUN | Tunisia | Tunézia |
| TUR | Turkiye | Törökország |
| URU | Uruguay | Uruguay |
| USA | USA | USA |
| UZB | Uzbekistan | Üzbegisztán |

## Testing

- **Unit (`teamDisplayName`)**: catalogue hit (HU), miss → English fallback,
  unknown locale → English fallback.
- **Unit (`teamIndex`)**: returns localised names for HU; returns English for a
  team absent from the catalogue; produces new objects (immutability).
- **Existing `displayMode.test.ts`**: unchanged — it builds English maps directly
  and never goes through `teamIndex`, so it stays green.
- **E2E (Playwright)**: switch the language picker to Hungarian on a page that
  shows resolved team names, assert a known team renders in Hungarian (e.g.
  `Horvátország`), then switch back to English and assert `Croatia`. (Per the
  project rule: frontend work is verified with an end-to-end test, not just
  build + lint.)

## Risks / notes

- `Locale` is declared in `web/src/i18n/strings.ts` (`type Locale = 'en' | 'hu'`);
  `teamNames.ts` imports it from `./strings`.
- Sorting in `AdminTeams` switches to locale-aware ordering of localised names —
  intended.
- Flag assets are keyed by ISO/short code and are unaffected; only the text and
  its `alt`/`title` localise.
