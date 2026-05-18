# 12 — H2H tiebreak not recomputed per still-tied subgroup

Status: done
Severity: HIGH
Area: crates/domain

## Problem

The head-to-head tiebreak (`crates/domain/src/scoring.rs:239-282`) sorts a
points-tied group by H2H stats, but `resolve_sub_group` keeps using the *same*
`h2h_stats` — computed over the original tied group — for any still-tied
sub-subgroup.

The FIFA rule is recursive: after an H2H mini-table partially resolves a
3-way tie, the ladder must restart for the remaining tied teams with H2H
recomputed *among only those teams*. As written, a partially-resolving 3-way
tie can produce wrong standings.

No test exercises a partially-resolving 3-way tie — the `rank_group_*` tests
all use groups that resolve cleanly on points.

## Expected

Recompute H2H stats for each still-tied subgroup before re-applying the GD /
goals H2H steps. Confirm the exact ladder against `FWC26_RULES.md` §2 — hence
`ready-for-human` for the rule confirmation.

## Acceptance

- Test: a 3-way tie that partially resolves via H2H produces the correct
  ordering.
- `cargo test -p domain` green.

## Decision (grilled 2026-05-17)

**Strict FIFA reapplication.** Fix `domain::rank_group` so that when the
head-to-head step separates some of a tied set but leaves a subset still tied,
the **whole ladder is reapplied to that subset from step 1** — with H2H
recomputed among *only* the still-tied teams (their games against each other).
Recursive; matches modern FIFA and `SCORING.md` §4 "among the tied teams" read
literally.

This now also governs **bracket resolution** — `fwc26` delegates to
`domain::rank_group` (see issue 19) — so a wrong sub-tie would send the wrong
team to the knockouts. Fixing it here fixes both.

## Comments

Rewrote the tiebreak helpers in `crates/domain/src/scoring.rs` as a recursive
ladder (`rank_tied` → `rank_h2h` → `rank_h2h_rung` → `rank_all_match`). When an
H2H rung separates part of a tied set, each strictly-smaller still-tied subset
re-enters `rank_tied` at step 1, so its H2H table is recomputed among *only*
those teams. `rank_group`'s public signature is unchanged. Added a failing-then-
passing regression test (`rank_group_h2h_partially_resolves_subgroup_recomputes_h2h`)
covering a 3-way tie that partially resolves on H2H GD. `cargo test -p domain`
green (43); clippy clean.
