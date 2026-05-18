# 06 — PRED-03 not enforced: group locks with partial / missing scores

Status: done
Severity: HIGH
Area: crates/api

## Problem

`SCENARIOS.md` PRED-03: "a match can only be locked when both scores are
filled". `submitGroup` with `lock: true` (`crates/api/src/gql/mutation.rs:78-89`)
locks whatever `MatchPredictionInput` rows are supplied — a group submitted with
only some of its games gets a partial lock with no completeness validation.

The input type also cannot represent "no prediction" distinct from `0-0`:
`MatchPredictionInput.home_score`/`away_score` are non-optional `i32`
(`gql/inputs.rs`).

## Expected

`lock: true` requires a prediction for every game in the group. Decide how the
input type represents an absent prediction (likely: a lock submission must
include all games).

## Acceptance

- API test: `submitGroup` with `lock: true` and missing games is rejected.
- `cargo test -p api` green.

## Comments

Fixed in `crates/api/src/gql/mutation.rs`: `submit_group` with `lock: true` now
requires a prediction for every game in the group, else it errors listing the
missing game ids. An existing test was updated to supply both games. Tests added
to `graphql.rs`.
