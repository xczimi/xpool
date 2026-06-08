# Country name variants (a team has more than one "full name")

Status: needs-triage
Area: domain (`Team`) + web display + i18n

## Idea

A national team currently carries a single `name: String`
(`crates/domain/src/model.rs` `Team`). But a country doesn't have *one* full
name — it has several, and "Hungary" is the canonical example:

- **Short / common name:** `Hungary`
- **Official long-form name:** `Hungary` today, but historically the same FIFA
  entity has competed as `Hungarian People's Republic` etc. — official forms
  drift over time while the team identity is stable.
- **Endonym (native name):** `Magyarország` / the team as `Magyar` (already
  appears in `web/src/i18n/strings.ts`).
- **Localised exonyms:** the name differs per UI language (EN `Hungary`,
  HU `Magyarország`), which our EN/HU i18n already cares about.

So "the name of a team" is context-dependent (display language, short vs full,
common vs official), and one `String` flattens all of that.

## Motivation

- We're i18n-first (EN + HU). A single English `name` can't render correctly in
  Hungarian, and team names are exactly the strings a Hungarian audience will
  notice if they're wrong.
- Tournaments span eras/contexts; locking to one "official" string bakes in a
  point-in-time choice.
- Display surfaces want different lengths (scoreboard column vs match header) —
  short code, common name, and full name serve different layouts.

## Sketch

- Decide whether name variants belong on `Team` (note: `model.rs` is a **locked
  contract** — additive `Option` fields only, ripples to every crate) or in a
  side lookup keyed by `short_code` / `external_id`, so the locked domain type
  stays minimal.
- Distinguish the axes that are getting conflated: **register** (common vs
  official/full) vs **language** (exonym per UI locale) vs **length** (display
  short code). These are independent and shouldn't collapse into one field.
- Keep `short_code` as the stable, language-neutral identity; resolve a
  display name from `(short_code, locale, register)`.
- Wire localised names through `web/src/i18n/strings.ts` rather than the
  imported tournament JSON, so translations live with the rest of i18n.

## Open questions

- Do we actually need the official long-form name anywhere in the UI, or only
  common + endonym? (Scope: is this i18n, or i18n + historical naming?)
- Source of localised names — hand-maintained in `strings.ts`, or pulled from a
  dataset keyed by FIFA/ISO code?
- Is Hungary a one-off or the first of many (Czechia/Czech Republic, North
  Macedonia, Türkiye…)? If general, model it generally rather than special-case.
