# Fun, fast mobile prediction entry — per group

Status: done — shipped via backlog-parallel-build (cluster/mytips-nav), merged to master 2026-06-27
Area: web

## Idea

Make entering predictions on a phone feel fun, simple, and fast — ideally
focused one group at a time rather than a long scrolling form.

## Motivation

Most people will fill in their tips on their phone, in a couple of spare
minutes, for several groups in a row. The experience should reward that: quick
to complete, satisfying to tap through, hard to get wrong — not a dense desktop
table squeezed onto a small screen. Friction here directly costs participation.

## Sketch

- **Per-group focus.** Show one group at a time; swipe / "Next group" to move on,
  with a clear sense of progress across the tournament (e.g. "Group C · 3 of 12").
- **Big, thumb-friendly score entry.** Replace the tiny `<select>` 0–9 dropdowns
  in `GroupTipForm`'s `ScoreInput` with large tap targets / steppers sized for
  thumbs. Minimise taps per match.
- **Instant, no-reload feedback.** Autosave drafts as you go; show a per-group
  "saved / N of M predicted" state so it never feels like work-in-progress can
  be lost.
- **A bit of delight.** Team flags (see [[../theme-switcher]] / the country-flags
  work), light micro-interactions on entry, a satisfying "group complete" beat.
- **Deadline-aware.** Surface how long until the group locks in human terms
  (ties to [[timezone-clarity]]'s relative-deadline idea), and make a locked /
  past-deadline group obviously read-only.

## Resolved decisions (2026-06-27 grill)

- **Swipe one-group-per-screen** (Peter's pick): card per group with progress
  ("Group C · 3 of 12"), swipe / "Next group" to advance. The fun, focused flow.
- **Score entry:** big steppers (+/−) replacing the tiny 0–9 `<select>` in `ScoreInput`.
- **Autosave drafts** as you go; per-group "saved / N of M predicted" state.
- **Deadline-aware:** locked / past-deadline groups are obviously read-only.
- **Knockout/draw-order entry:** desktop-only this round (the swipe flow targets the
  group-stage score-entry case).
- **Relationship to `GroupTipForm`:** a mobile-specific view/flow, not just a responsive
  tweak of the desktop table.
- Cluster: `cluster/mytips-nav` (Wave 2), built AFTER [[knockout-subgroup-anchors]].

## Related

- [[fixed-width-display]] — mobile entry depends on getting the responsive
  layout right.
- [[timezone-clarity]] — relative deadline countdowns belong in this flow.
- The country-flags work (PR #12) supplies the flags that make it feel alive.
