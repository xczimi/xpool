# PR #3 code review — findings backlog

Issues raised from the code review of PR #3 (`feat/local-build`). Each file
under `issues/` is one finding. Numbering: `01`–`03` CRITICAL, `04`–`14` HIGH,
`15`–`23` MEDIUM (the original review); `24`–`31` are a second-pass review of
the remediation + round-nav delta.

Issues `01`–`23` are all **done** — the 17 original `ready-for-agent` issues
plus the 6 `ready-for-human` issues (02, 09, 12, 18, 19, 23) grilled on
2026-05-17, each with a recorded `## Decision`.

Issues `24`–`31` are the **second-pass review** (2026-05-18) of the 13 commits
since the first review — all `ready-for-agent`, open.

| #  | Severity | Area    | Status         | Title |
|----|----------|---------|----------------|-------|
| 01 | CRITICAL | api     | done           | submitGroup enforces no deadline / locking-is-final rule |
| 02 | CRITICAL | api     | decided         | enter_result can silently overwrite a locked official result |
| 03 | CRITICAL | storage | done           | put_player optimistic-concurrency check is a no-op |
| 04 | HIGH     | api     | done           | scoreboard(pool) leaks pool privacy to non-members |
| 05 | HIGH     | api     | done           | invite has no authorization or self-target guard |
| 06 | HIGH     | api     | done           | PRED-03 not enforced — group locks with partial scores |
| 07 | HIGH     | storage | done           | scan_prefix does not paginate — silent truncation at 1 MB |
| 08 | HIGH     | storage | done           | ensure_table does not wait for the table to become ACTIVE |
| 09 | HIGH     | storage | decided         | storage layout diverges from DATA_MODEL.md §9 |
| 10 | HIGH     | fwc26   | done           | determine_winner_loser treats a knockout draw as a home win |
| 11 | HIGH     | domain  | done           | scoring.rs reintroduces the clock seam (unwrap_or_else(Utc::now)) |
| 12 | HIGH     | domain  | decided         | H2H tiebreak not recomputed per still-tied subgroup |
| 13 | HIGH     | web     | done           | no unit/integration tests for web/src |
| 14 | HIGH     | web     | done           | standings.ts mutates TeamStats objects in place |
| 15 | MEDIUM   | api     | done           | negative / oversized scores clamped instead of rejected |
| 16 | MEDIUM   | api     | done           | create_pool accepts a client-supplied id with no collision check |
| 17 | MEDIUM   | api     | done           | updateProfile does not validate nick / full_name |
| 18 | MEDIUM   | api     | decided         | recompute failure after enter_result leaves inconsistent state |
| 19 | MEDIUM   | fwc26   | decided         | group-stage tiebreaker simplified (no H2H, conduct=0, hardcoded map) |
| 20 | MEDIUM   | storage | done           | delete_table / test_repo share one fixed table name |
| 21 | MEDIUM   | web     | done           | rounds.ts round labels hardcoded in English, not i18n'd |
| 22 | MEDIUM   | web     | done           | AdminResults score input has no validation; e2e-stack.sh parses JSON with node |
| 23 | MEDIUM   | domain  | decided         | score_leaf_group hardcodes per-match `complete = true` |
| 24 | HIGH     | api     | ready-for-agent | enterResult accepts `advancer` but never persists it |
| 25 | HIGH     | storage | ready-for-agent | put_player OCC is not atomic; adapters diverge under concurrency |
| 26 | HIGH     | web     | ready-for-agent | AdminResults unlock succeeds with no confirmation |
| 27 | MEDIUM   | api/domain | ready-for-agent | deadline boundary comparator inconsistent (`>=` vs `>`) |
| 28 | MEDIUM   | web     | ready-for-agent | round-tab state can desync from the selected group |
| 29 | MEDIUM   | web     | ready-for-agent | useMemo dependencies are the whole `tournament` object |
| 30 | MEDIUM   | web     | ready-for-agent | AdminResults follow-up refetches are unguarded |
| 31 | MEDIUM   | web     | ready-for-agent | All Tips knockout grid rebuilds derived data every render |
