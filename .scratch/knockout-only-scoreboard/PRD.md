# Knockout-only scoreboard — a re-entry point for late joiners

Status: done — shipped via backlog-parallel-build (cluster/standings), merged to master 2026-06-27
Area: web (+ possibly domain scoring)

## Idea

Add a scoreboard scoped to the **knockout stage only**, separate from the
overall (group + knockout) standings. Players who missed the group-stage
deadlines — and so fell out of contention overall — can rejoin and compete on a
clean slate from the Round of 32 onward.

## Motivation

The overall scoreboard punishes anyone who didn't lock all their group-stage
predictions in time; once you're far behind, engagement drops. A knockout-only
board resets the race for the back half of the tournament, giving late joiners a
real reason to re-engage and keep predicting. It's a retention feature as much
as a scoring one.

## Sketch

- A scoreboard view that sums points from knockout-stage matches only.
- Probably a filter/toggle on the existing scoreboard (Overall ⇄ Knockout only)
  rather than a whole new page.
- Reuse the existing scoring; just restrict the match set to the knockout subtree
  (the domain already knows group vs knockout structure).
- Pairs with [[knockout-tip-labels]] and the deadline-reminder nudges
  ([[ses-deadline-reminders]]) to pull lapsed players back in for the knockouts.

## Resolved decisions (2026-06-27 grill)

- **No entry barrier (Peter's clarification):** players are NOT blocked from entering
  knockout predictions because they missed group tips. KO tips are entered normally,
  each per its own match deadline. "Late" only ever meant missing *group-stage* tips.
  This feature is therefore a **re-engagement VIEW**, not an entry-policy change — no
  domain deadline/entry change needed.
- **Scoring:** re-sum points from knockout-stage matches only, fresh from zero.
- **Surfacing — BOTH:** a toggle on the existing `ScoreboardPage` (Overall ⇄
  Knockout-only) AND a standalone linkable route (e.g. `/scoreboard/knockout`).
- **Pool-scoped** (consistent with the overall board; follows existing pool selection).
- **Ties/start:** everyone starts KO at zero; tie-break reuses the overall board's
  ordering rules.
- Cluster: `cluster/standings` (Wave 1). Owns `ScoreboardPage.tsx`; adds one resolver
  in `query.rs`; minimal `domain`/`fwc26` filtering to the knockout subtree.
