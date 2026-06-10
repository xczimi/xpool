# Share templates on /pools + Home FAQ

**Date:** 2026-06-10 · **Branch:** `pools-share-templates`

## Problem

`docs/share-copy.md` holds ready-to-send invite messages (WhatsApp, one-liner,
colleague email, Hungarian) but they live only in the repo. A pool member
inviting friends has to find that file. Surface the messages in the app so they
can be copied straight from the browser, and fold the explanatory bits
(objection-handling, scoring summary) into the Home page.

## Decisions (from brainstorming)

- **Placement:** a standalone "Share templates" panel on `/pools`, pool-agnostic,
  showing the messages with the literal `{LINK}` placeholder (not a real injected
  invite link — the panel isn't tied to one minted invite).
- **Variants:** the curated four — Short (WhatsApp), One-liner, Email
  (colleagues), Hungarian. The objection/scoring blocks become Home content, not
  share buttons.
- **FAQ:** fold "isn't that a lot of predictions?" and "scoring at a glance" into
  the Home how-it-works section (not Rules).
- **Message language ≠ UI language.** The share variants are shown regardless of
  the en/hu toggle — the inviter picks by *recipient*, so the Hungarian message
  is always available. Only the surrounding chrome (labels, hint, button) is
  i18n'd.

## Design

### Part 1 — `/pools` share panel

- **`web/src/content/shareTemplates.ts`** — the four message bodies as plain-text
  constants (markdown emphasis stripped so they paste clean). `{LINK}` stays
  literal. Exported as `SHARE_TEMPLATES: { id, labelKey, body }[]`. Bodies are
  fixed content (3 English, 1 Hungarian); labels come from i18n by `labelKey`.
  Single source of truth for these four; `docs/share-copy.md` gets a pointer.
- **`web/src/components/ShareTemplates.tsx`** — a collapsible `<details>` panel
  rendered once at the bottom of `PoolsPage`. Per template: i18n label, body in a
  selectable `<pre>`, and a **Copy** button (`navigator.clipboard.writeText`) with
  a short-lived "Copied!" state (local `copiedId` state). A one-line hint:
  "paste your invite link where it says {LINK}".

### Part 2 — Home how-it-works FAQ

- Two Q&A items appended to `HomePage`'s how-it-works block, compact, linking to
  the existing Rules page:
  - *Isn't that a lot of predictions?* — one group at a time, each pick due
    before its own match; not fantasy football.
  - *Scoring at a glance* — +1 home · +1 away · +2 result (≤4/match), knockout
    multipliers, group-order bonus, picks lock at kickoff.
- UI content → full **en + hu** in `strings.ts`.

### i18n keys (en + hu)

`shareTemplatesTitle`, `shareTemplatesHint`, `shareTemplateShort`,
`shareTemplateOneLiner`, `shareTemplateEmail`, `shareTemplateHungarian`,
`copied`; Home: `homeFaqTitle`, `homeFaqQ1`, `homeFaqA1`, `homeFaqQ2`,
`homeFaqA2`. Reuse `copyLink` for the button.

## Testing

- **Unit** (`shareTemplates.test.ts`): every body contains `{LINK}`; ids unique;
  four entries. Matches the pure-function test style in this codebase.
- **E2E** (Playwright): dev-stub auth reaches `/pools` as a seeded player — log
  in, open the panel, assert a template body renders. Assert visible text, not
  clipboard (Playwright clipboard is permission-flaky).
- `tsc -b` build, eslint, full vitest green before merge.

## Out of scope

- Injecting a real invite link into the copy (panel is pool-agnostic).
- Full Hungarian parity of the one-liner/email variants (recipient-language, not
  UI-language).
- Rules-page changes.
