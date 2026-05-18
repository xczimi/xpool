# 25 — put_player OCC is not atomic; adapters diverge under concurrency

Status: done
Severity: HIGH
Area: crates/storage

## Problem

`DynamoRepository::put_player` (`crates/storage/src/dynamo.rs:363-395`) does a
*separate* `get_item` to choose between `attribute_not_exists(pk)` (new player)
and `#ver = :v` (update), then issues a conditional `put_item`. The branch
decision rests on a stale read — the item can change between the `get` and the
`put`. First-write races are still rejected correctly, but the two-call design
is not atomic (e.g. a delete between read and put would spuriously fail the
`#ver` branch).

`InMemoryRepository::put_player` holds its lock across the whole check+insert
and *is* atomic — so the two adapters are not equivalent under concurrency,
which issue 03's acceptance ("enforce it identically") intended.

The issue-03 concurrency tests (`*_concurrent_writes_second_conflicts`) issue
`put_player` calls *sequentially* — they verify the OCC condition but never
exercise true interleaving, so this window is untested.

## Expected

Make the Dynamo write a single atomic conditional put — one `put_item` whose
condition expression is `attribute_not_exists(pk) OR #ver = :expected` — so no
prior `get_item` is needed and the check is atomic with the write.

## Acceptance

- `put_player` issues one conditional `put_item`, no preceding `get_item`.
- A test exercising the new-vs-update branches still passes; the OCC-conflict
  test still passes.
- `DYNAMO_TEST=1 cargo test -p storage` green.

## Comments

Removed the preceding `get_item` in `DynamoRepository::put_player`; the write
is now a single atomic conditional `put_item` with condition
`attribute_not_exists(pk) OR #ver = :v`, where `:v` is the caller-supplied
(old) version. The new-vs-update branch is the condition itself — no stale
read — and the issue-03 OCC model (persist `version + 1`) is unchanged. Added
`dynamo_player_second_insert_of_existing_id_conflicts` proving a second insert
of an existing id is rejected by the condition. All 14 dynamo integration
tests pass with `DYNAMO_TEST=1` on a fresh table; `cargo clippy -p storage
--tests -- -D warnings` is clean. (Pre-existing cross-run pollution on the
shared `xpool-test` table can cause fixed-id tests to fail on a re-run — drop
the table between runs; unrelated to this fix.)
