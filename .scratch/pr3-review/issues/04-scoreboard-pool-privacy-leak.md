# 04 — scoreboard(pool) leaks pool privacy to non-members

Status: done
Severity: HIGH
Area: crates/api

## Problem

The `scoreboard` query (`crates/api/src/gql/query.rs:71-80`) is public — no
`CurrentPlayer::require`. Passing an arbitrary `pool` id either returns that
pool's filtered scoreboard / member list or errors `"pool not found"`.

A non-member can therefore (a) enumerate valid pool ids and (b) read any
pool's member list and scores. Pool membership is meant to be private
(`gql/types.rs:289` notes join codes are "members only").

## Expected

`scoreboard` with a `pool` filter requires authentication and pool membership.
The global (no-pool) scoreboard can stay public if intended.

## Acceptance

- API test: non-member querying `scoreboard(pool:)` is rejected; member
  succeeds.
- `cargo test -p api` green.

## Comments

Fixed in `crates/api/src/gql/query.rs`: `scoreboard` with a `pool` filter now calls
`CurrentPlayer::require` and rejects callers who are not a member/owner of that
pool. The global (no-pool) scoreboard stays public. Tests added to `graphql.rs`.
