# One-liner intro on every page

Status: needs-triage
Area: web / content

## Idea

Give each page a simple one-line explanation of what it's for, shown near its
title — so users always know where they are and what the page does.

## Motivation

The app has many pages (Home, My Tips, Schedule, Today, Scoreboard, All Tips,
Perfect, Pools, Invite, Rules, Profile, Admin). A newcomer shouldn't have to
infer a page's purpose from its contents. A single friendly sentence under each
title removes that guesswork and sets the tone.

## Sketch

- A consistent subtitle/intro slot under each page's heading (one shared pattern,
  not bespoke per page).
- One short i18n'd sentence per page in `web/src/i18n/strings.ts` (EN + HU),
  keyed alongside the existing per-page title keys.
- Examples (final wording TBD):
  - My Tips — "Enter and edit your score predictions, group by group."
  - Scoreboard — "Who's winning — total points across the pool."
  - Today — "Matches kicking off today and their deadlines."
  - Perfect — "Your spot-on predictions."

## Open questions

- Always visible, or dismissible / first-visit-only to avoid clutter for
  regulars?
- Does it double as the page's `<title>` / meta description for shareable links?

## Pages to cover

`HomePage`, `MyTipsPage`, `SchedulePage`, `TodayPage`, `ScoreboardPage`,
`AllTipsPage`, `PerfectPage`, `PoolsPage`, `InvitePage`, `InviteClaimPage`,
`RulesPage`, `ProfilePage`, `AdminPage`.

## Related

- [[rules-content]] — the Home and Rules one-liners coincide with that work.
