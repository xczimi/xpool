# 09 — storage layout diverges from DATA_MODEL.md §9

Status: done
Severity: HIGH
Area: crates/storage / .specs

## Problem

`DATA_MODEL.md` §9 specifies a partition-key + sort-key design (e.g.
`<t>#PLAYER` partition / `<playerId>` sort key) so that "all players = one
`Query` of the `<t>#PLAYER` partition".

The implementation (`crates/storage/src/dynamo.rs`) uses a flat single-PK
scheme (`<t>#PLAYER#<playerId>`) and a table-wide `Scan` with `begins_with`.
The module doc acknowledges the deviation, but `.specs/` is the stated source
of truth. A `Scan` is more expensive and reads/filters every item in the table
(Persons, Identities, Tournaments, Pools) to list players.

## Expected — decision needed

Either:
- Update the implementation to the PK+SK design the spec mandates and use
  `Query`; or
- Update `DATA_MODEL.md` §9 (and add an ADR / corrections note) to ratify the
  flat-PK + `Scan` design as the deliberate choice.

Human decision required — hence `ready-for-human`.

## Acceptance

- Spec and code agree, with the rationale recorded.

## Decision (grilled 2026-05-17)

**Align the code to the spec.** Refactor `DynamoRepository` to the PK+SK
scheme of `DATA_MODEL.md` §9:

- Player — PK `<t>#PLAYER`, SK `<playerId>`; `list_players` = `Query` on the
  partition (not `Scan`).
- Pool — PK `<t>#POOL`, SK `<poolId>`; `list_pools` = `Query`.
- Person `PERSON#<id>`, Identity `IDENTITY#<provider>#<providerId>`,
  Tournament `<t>#TOURNAMENT`, Scoreboard `<t>#SCOREBOARD` stay single-PK
  items (unchanged).

No data migration — pre-launch, DynamoDB Local is in-memory and re-imported
each run. The issue-07 pagination loop moves from `scan_prefix` onto the new
`Query` paths. `InMemoryRepository` and the storage tests update to match.
`.specs/DATA_MODEL.md` §9 stays the source of truth, unchanged.

## Comments

Refactored `DynamoRepository` to the composite-key (`pk` + `sk`) scheme.
`ensure_table` now defines both attributes and a Hash+Range key schema.
Player items live in partition `<t>#PLAYER` keyed by `<playerId>`, Pool in
`<t>#POOL` keyed by `<poolId>`; `list_players`/`list_pools` are now a `Query`
of one partition (no more table-wide `Scan`). Single-instance items
(Person/Identity/Tournament/Scoreboard) keep a constant `sk = "#"`. The
issue-07 1 MB pagination loop moved onto the `Query` path. Added two storage
integration tests: namespace isolation (a tournament's `list_players` returns
only its own players, not other namespaces or Pools) and Query pagination past
one 1 MB page. All 13 gated dynamo tests pass against DynamoDB Local; clippy
clean. `InMemoryRepository` needed no change (HashMap fake).

