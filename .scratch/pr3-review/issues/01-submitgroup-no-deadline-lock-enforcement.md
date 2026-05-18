# 01 — submitGroup enforces no deadline / locking-is-final rule

Status: done
Severity: CRITICAL
Area: crates/api

## Problem

`submitGroup` (`crates/api/src/gql/mutation.rs:121-172`, via
`apply_group_predictions` at `mutation.rs:62-113`) unconditionally drops all
existing predictions for the group's games and re-adds the input. There is no
check of `now` against `tournament.deadline(group_id)`, and no check of the
existing `locked` flag.

Consequences:
- A player can **overwrite an already-locked prediction**.
- A player can **submit/edit predictions after the group deadline has passed**.

`effective_locked` only protects *scoring*, not persisted state — it cannot
undo an overwritten lock. This contradicts `API.md` §6 ("locking is final for
the player") and the `SCENARIOS.md` PRED-* deadline rules.

## Expected

- Reject edits to a group whose deadline has passed (use the request `now`
  already threaded into the resolver context).
- Reject overwriting a prediction that is already `locked`.

## Acceptance

- New API tests: post-deadline `submitGroup` is rejected; overwriting a locked
  prediction is rejected.
- `cargo test -p api` green.

## Comments

Fixed in `crates/api/src/gql/mutation.rs`: `submit_group` now rejects edits once the
group deadline has passed (server `now` vs `tournament.deadline`) and rejects any
submit that would overwrite an already-`locked` prediction. Tests added to
`graphql.rs`.
