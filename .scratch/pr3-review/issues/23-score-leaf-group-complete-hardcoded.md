# 23 — score_leaf_group hardcodes per-match `complete = true`

Status: ready-for-agent
Severity: MEDIUM
Area: crates/domain

## Problem

`score_leaf_group` (`crates/domain/src/scoring.rs:535`) sets `complete = true`
for every `MatchPrediction`. The reasoning (a `MatchPrediction` always has both
`u8` scores) is locally sound, but it means a player who predicted *some* games
in a `LockTogether` group but not others still gets each individual prediction
counted post-deadline.

`DATA_MODEL.md` §7 says "a complete draft auto-counts; an incomplete one scores
0" — "complete" there is arguably *group-level* (all matches predicted), not
per-match. The engine treats it per-match.

## Expected — decision needed

Confirm with the spec author whether "complete" is group-level or per-match,
then either implement group-level completeness or document the per-match
interpretation in `scoring.rs` and `SCORING.md`. Human decision — hence
`ready-for-human`.

## Acceptance

- Interpretation recorded in `.specs/`; code matches it; covered by a test for
  a partially-predicted `LockTogether` group.

## Decision (grilled 2026-05-17)

**Per-match interpretation confirmed — documentation-only, no behaviour
change.** Each `MatchPrediction` is its own draft; it always carries both
scores, so it is always "complete". A partially-predicted, unlocked group
auto-counts the games that were filled after the deadline (the rest score 0).

Action: replace the deliberation comment at `scoring.rs:533-535` with a clear
statement of the per-match rule, and pin it in `SCORING.md` / `DATA_MODEL.md`
§7 so the apparent "complete draft / incomplete draft" conflict is resolved in
writing. (Note: issue 06 already makes an explicitly *locked* group require all
games, so the `complete` flag only governs the post-deadline auto-count path.)
