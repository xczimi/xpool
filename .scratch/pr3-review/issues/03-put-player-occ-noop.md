# 03 — put_player optimistic-concurrency check is a no-op

Status: done
Severity: CRITICAL
Area: crates/storage

## Problem

`DynamoRepository::put_player` (`crates/storage/src/dynamo.rs:268-317`) issues a
conditional put with `#ver = :v` where `:v` is `p.version` — the *new* version
the caller supplies — and nothing ever increments `version`.

Result: two writers who both read version 0 both write version 0 and both
succeed. OCC provides no protection against lost updates.

The trait doc says "the caller must bump version on write", but the test
`dynamo_player_optimistic_concurrency_success` writes `version = 0` twice and
passes — it would pass even with OCC entirely removed. The `InMemoryRepository`
fake has the same flaw (`existing.version == p.version`).

## Expected

Pick one model and apply it consistently across both adapters + the trait doc:
- Repo bumps and stores `version + 1`, conditions the put on `stored == old`; or
- Caller bumps `version`, the put conditions on `stored == p.version - 1`.

`DATA_MODEL.md` §9 specifies OCC against the *stored* version — align with it.

## Acceptance

- A real concurrency test: two writes from the same base version — second
  fails with a version-conflict error.
- Both `DynamoRepository` and `InMemoryRepository` enforce it identically.
- Trait doc updated; `cargo test -p storage` (and `DYNAMO_TEST=1`) green.

## Comments

Adopted the "repo owns the version" model: `put_player` now conditions the
conditional write on the caller-supplied (old) version and persists `version + 1`
in both `DynamoRepository` and `InMemoryRepository`. Trait doc (`lib.rs`) updated.
Added real concurrency tests (two writers from one base version; second fails)
to `tests/memory.rs` and `tests/dynamo.rs`. No ripple into api/xtask — every
caller already passes the version it last read; `cargo test --workspace` green.
