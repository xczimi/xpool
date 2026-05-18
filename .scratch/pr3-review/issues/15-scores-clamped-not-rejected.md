# 15 — negative / oversized scores clamped instead of rejected

Status: done
Severity: MEDIUM
Area: crates/api

## Problem

`MatchPredictionInput` / result inputs use `home_score: i32` and accept
negatives and values > 255. The mutations apply `.clamp(0, u8::MAX)`
(`crates/api/src/gql/mutation.rs:85-86`, `mutation.rs:347-348`,
`gql/inputs.rs:8-10`), so `-3` silently becomes `0` with no client feedback.

The project's input-validation rules require rejecting malformed input, not
coercing it.

## Expected

Reject out-of-range scores with a validation error instead of clamping. Define
a sensible upper bound.

## Acceptance

- API test: a negative or oversized score is rejected.
- `cargo test -p api` green.

## Comments

Added `validate_score` + `MAX_SCORE` (99) in `crates/api/src/gql/inputs.rs`;
`apply_group_predictions` and `enter_result` now reject out-of-range scores
instead of clamping. Tests added to `graphql.rs`.
