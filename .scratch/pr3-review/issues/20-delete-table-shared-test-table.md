# 20 — delete_table / test_repo share one fixed table name

Status: done
Severity: MEDIUM
Area: crates/storage

## Problem

`test_repo()` (`crates/storage/tests/dynamo.rs:33-46`) uses `XPOOL_TABLE` with
a fixed default (`xpool-test`) — it does **not** create a uniquely-named table.
`dynamo_delete_table_removes_it` (`tests/dynamo.rs:311-320`) deletes that
shared table; with tests running in parallel (the default), it can delete the
table out from under other in-flight tests.

The doc comment in `dynamo_delete_table_removes_it` claiming `test_repo()`
creates a uniquely-named table is wrong.

## Expected

Give each test (or at least the delete-table test) a uniquely-named table, or
serialize the delete-table test. Fix the misleading comment.

## Acceptance

- `DYNAMO_TEST=1 cargo test -p storage` is stable under parallel execution.

## Comments

Added `unique_table_repo(suffix)` test helper that builds a repo backed by a
freshly-named table (`xpool-test-<suffix>-<pid>`). `dynamo_delete_table_removes_it`
now uses it instead of the shared `xpool-test` table, so it can no longer delete
the table out from under parallel tests. Fixed the misleading comments in both
`test_repo` and the delete-table test.
