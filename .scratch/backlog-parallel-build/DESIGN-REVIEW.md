# Backlog parallel build — design review (2026-06-27)

Mid-altitude review of each idea before build. More than the one-line PRD idea,
less than the full TDD plan. Decisions come from the 2026-06-27 grill + stated
defaults. **Build is held until Peter approves this.**

---

## Wave 1

### cluster/match-page — `MatchPage.tsx`

**1. Sortable predictions.** Clickable column headers (player · prediction · points)
re-order the prediction list. Default order = scoreboard position; choice persists
in localStorage. Sort is client-side over already-revealed rows; only active once
tips are revealable. *Not doing:* server-side sort.

**2. Highlight self.** Your own row gets a background tint + a small "you" badge so
you find yourself instantly regardless of sort. *Not doing:* pin-to-top; applying it
to other lists (match page only this round).

**3. What-if (±1 goal).** Two extra columns — "if home scores" / "if away scores" —
showing each player's resulting points, delta emphasised (absolute shown too),
re-scored live in the browser. Pool-scoped; only once tips are revealable.
*Not doing:* arbitrary scoreline picker; any server resolver.

### cluster/standings — `ScoreboardPage.tsx` + domain

**4. Knockout-only scoreboard.** A board that re-sums knockout-stage points from zero,
so players behind on overall totals get a fresh race (re-engagement). Pure re-slice of
the existing per-round scoreboard — **no entry-policy/deadline change** (KO tips are
already entered per each match's own deadline). Surfaced as **both** an Overall⇄Knockout
toggle on the Scoreboard **and** a `/scoreboard/knockout` route. Pool-scoped; ties reuse
overall ordering.

### cluster/player-analytics — new pages + `App.tsx`

**5. Head-to-head.** `/h2h/:a/:b` (clean aliases, no UUIDs) comparing two players:
points, positions, total delta, and a per-match breakdown where they differ. Entry from
the scoreboard (pick two) and from a player page. Pool-scoped; reuses scoreboard data
client-side. *Not doing:* N-player compare.

**6. Points timeline chart.** Hand-rolled SVG line of cumulative points by round; can
overlay two players (powers H2H). No charting library. Computed client-side from existing
scoreboard stages. *Not doing:* rank/position axis, calendar-date x-axis (both deferred).

### cluster/backend-infra — `crates` mail + `infrastructure/*.tf` + `bin/`

**7. SES deadline reminders.** Emails via AWS SES. Three ways to fire:
- **1h last-call** before a group/match deadline (automated, hourly scan).
- **Daily matchday digest** at 00:00 America/Los_Angeles (automated, daily).
- **Manual admin "notify pool"** send (so you can test on dev).

Sent to *all* verified emails of players with *incomplete/unlocked* predictions; deduped
so nothing repeats; EN/HU templates. *Not doing:* opt-out/unsubscribe, 24h nudge, SES
bounce/complaint handling (all future work).

**8. local-dev `--fresh`.** Opt-in flag that loads the latest cached prod snapshot into
the *current branch's* table (fixes the master-vs-branch mismatch). *Not doing:* live
pull by default; blanking dev auth/clock.

---

## Wave 2 (after Wave 1 lands)

### cluster/mytips-nav — `pages/mytips/*`

**9. Knockout subgroup anchors.** Deep links like `/mytips/<round>#<group.id>` smooth-scroll
a group/match section into view (group stage + knockout); the round tab stays the only
route. Built first.

**10. Mobile prediction entry.** Swipe one-group-per-screen with progress ("Group C · 3 of
12"), big +/− steppers replacing the tiny 0–9 dropdowns, autosave, locked groups read-only.
A mobile-specific flow. *Not doing:* knockout/draw-order entry on mobile (stays desktop).

### cluster/cross-cutting-ux — every page

**11. Page one-liner intros.** A one-sentence subtitle under each page's heading, always
visible, EN/HU, via one shared `PageHeading` component across ~11 pages. Wording will be
drafted for your review. *Not doing:* dismissible/first-visit-only; doubling as meta/title.

---

## Settled outside build

- **fixed-width-display** — DROPPED this round (your call when asked).
- **timezone-clarity** — DONE; humane countdown already shipped the core.
