# 16 — create_pool accepts a client-supplied id with no collision check

Status: done
Severity: MEDIUM
Area: crates/api

## Problem

`create_pool` (`crates/api/src/gql/mutation.rs:176-197`) takes the pool `id`
from the caller and `put_pool` overwrites any existing pool with that id (no
existence check). A player can clobber another player's pool by reusing or
guessing an id.

## Expected

Either generate the pool `id` server-side, or check for collision + reject if
the id already exists.

## Acceptance

- API test: creating a pool with an already-used id is rejected (or ids are
  server-generated and the input no longer accepts one).
- `cargo test -p api` green.

## Comments

Fixed in `crates/api/src/gql/mutation.rs`: `create_pool` now checks `list_pools`
for the supplied id and rejects a duplicate before writing, preventing pool
clobbering. Test added to `graphql.rs`.
