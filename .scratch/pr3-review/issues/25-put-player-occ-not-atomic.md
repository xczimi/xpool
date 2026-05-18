# 25 — put_player OCC is not atomic; adapters diverge under concurrency

Status: ready-for-agent
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
