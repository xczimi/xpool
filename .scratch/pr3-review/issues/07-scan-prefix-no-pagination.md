# 07 — scan_prefix does not paginate, silently truncates at 1 MB

Status: done
Severity: HIGH
Area: crates/storage

## Problem

`DynamoRepository::scan_prefix` (`crates/storage/src/dynamo.rs:202-226`) issues
a single `Scan`. DynamoDB returns at most 1 MB per call and sets
`LastEvaluatedKey`. `list_players` / `list_pools` therefore silently return an
incomplete list once the data exceeds 1 MB — no error.

## Expected

Loop on `last_evaluated_key`, accumulating items until the scan is exhausted.

## Acceptance

- `scan_prefix` paginates fully.
- Test exercising > 1 page (or a unit test asserting the pagination loop runs).
- `DYNAMO_TEST=1 cargo test -p storage` green.

## Comments

`scan_prefix` now loops on `last_evaluated_key` via `set_exclusive_start_key`,
accumulating items until DynamoDB stops returning a continuation key. Added a
gated integration test (`dynamo_scan_prefix_paginates_past_one_page`) that
writes 60 padded players (~1.2 MB) in a private tournament namespace and asserts
all 60 come back — would fail if the loop were missing.
