# 10 — determine_winner_loser treats a knockout draw as a home win

Status: done
Severity: HIGH
Area: crates/fwc26

## Problem

`determine_winner_loser` (`crates/fwc26/src/lib.rs:619-630`) returns the home
team as the winner when a knockout match result is a draw. Knockout matches
cannot end drawn — the advancer is decided by the result user's
standings/penalty prediction. A wrong winner silently propagates through the
entire bracket (R32 → Final).

The group-tie path already consults the standings prediction
(`lib.rs:388-391`); the knockout path should do the equivalent.

## Expected

For a one-match knockout group ending level, resolve the advancer from the
result user's standings/penalty prediction for that group rather than
defaulting to the home team.

## Acceptance

- Test: a drawn knockout match resolves the advancer per the penalty/standings
  prediction.
- `cargo test -p fwc26` green.

## Comments

`determine_winner_loser` now takes the match's wrapping one-match-group id and
the result `Player`. On a level 90-minute score it reads the result user's
`StandingsPrediction` for that group (`ordering[0]` = the ET/penalty advancer),
mirroring the group-stage tie path; it falls back to the home team only when no
usable prediction exists. Added `test_resolve_knockout_draw_uses_standings_prediction`
in `crates/fwc26/tests/resolve_bracket_tests.rs` (drawn M73 advances B3, the
predicted advancer, not home team A3). Signature is private to the crate — no
public API change. `cargo test -p fwc26` and `cargo clippy -p fwc26 --tests` green.
