# 08 — ensure_table does not wait for the table to become ACTIVE

Status: done
Severity: HIGH
Area: crates/storage

## Problem

`ensure_table` (`crates/storage/src/dynamo.rs:86-116`) calls `create_table` and
returns while the table is still `CREATING`. An immediate `put_item`/`get_item`
can fail with `ResourceNotFoundException`. DynamoDB Local is fast enough to
usually mask this; against real AWS it is a flaky-failure source.

## Expected

After `create_table`, poll `describe_table` until `TableStatus::Active` (or use
the SDK's table-exists waiter).

## Acceptance

- `ensure_table` does not return until the table is `ACTIVE`.
- `DYNAMO_TEST=1 cargo test -p storage` green.

## Comments

`ensure_table` now calls a new `wait_for_active` helper that polls
`describe_table` until `TableStatus::Active` (60 × 500 ms ceiling, then errors).
Runs after both a fresh create and the idempotent already-exists path, so the
repository is safe to use the instant `ensure_table` returns. Gated tests green.
