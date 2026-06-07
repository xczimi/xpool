# rustfmt determinism — design

**Status:** ON HOLD (2026-06-07) — paused until the parallel `fix-auth0-access-token-email`
session finishes, to avoid reformat collisions on shared auth files.

## Problem

> "My local rustfmt formats differently from the repo's committed style
> (collapsing/expanding arrays and arg lists)."

The same source reformats differently depending on whose machine runs `cargo fmt`,
producing spurious diffs that fight real changes.

## Root cause (evidenced)

The repo pins **nothing** about formatting:

- No `rust-toolchain.toml` — nothing pins which rustfmt binary runs.
- No `rustfmt.toml` — the committed style is "whatever rustfmt the last committer ran";
  no option intent is declared.
- No CI, no git hooks, no pre-commit — nothing catches drift before it lands.

Measured on 2026-06-07 in a worktree off `origin/master` (base `8e23b8e`):

- Both locally-installed toolchains report **rustfmt 1.9.0-stable** yet `cargo fmt --check`
  shows **112 diff locations** against committed code → the repo was last formatted by an
  **older** rustfmt. This is genuine cross-version drift, not a local misconfig.
- The drift is bidirectional (rustfmt's line-fitting heuristics changed across versions):
  - some chains that were one line are now expanded
    (`header.kid.ok_or_else(...)` → 3 lines),
  - some that were multi-line now collapse
    (`...URL_SAFE_NO_PAD.encode(mac.finalize()...)` → 1 line;
    `verify_token(trust, token)` args → 1 line).

So the fix is **determinism**: everyone, every machine, and future-me run the *same*
rustfmt and produce byte-identical output.

## Approved approach: pin + gate

Three independent levers; they stack:

1. **`rust-toolchain.toml`** — pin the toolchain (channel/version) so `cargo fmt` uses the
   *same rustfmt binary* everywhere. This is the load-bearing lever — it makes output
   deterministic even for unconfigured options.
2. **`rustfmt.toml`** — declare style intent explicitly (at minimum the options whose
   defaults drift). Documents the choice; reduces surprise on future version bumps.
3. **`cargo fmt --check` gate** — catch drift before it lands. The repo has no CI today, so
   this means adding a minimal GitHub Actions workflow (and/or a `bin/` guard / pre-commit
   hook). Open sub-decision at resume time — see below.

A gate only passes if the repo is already clean against the pinned version, which requires
a **one-time repo-wide reformat** (the 112 locations). That sweep is the only step with
collision risk.

## Sequencing decision

The active `fix-auth0-access-token-email` branch overlaps the auth files where many of the
112 diffs live. Options weighed: (a) config now / sweep after merge, (b) full sweep now,
(c) hold. **Chosen: HOLD** until the parallel session is done, to avoid duplication and
cross-branch conflicts.

When resuming, the likely order is: confirm the parallel work has merged → land config →
single clean fmt sweep on an up-to-date base → add the gate.

## Open item for resume: which version to pin

The committed style matches **no** currently-installed rustfmt, so zero-churn is not free:

- **A (recommended): pin current stable (rustfmt 1.9.0) and accept the one-time reformat.**
  Simple, forward-looking, one bounded sweep.
- **B: hunt for the older rustfmt that produced the committed style** to avoid churn.
  Fragile — the repo may be internally inconsistent (formatted by several versions over its
  history), so no single older version is guaranteed clean. Not worth the effort.

## Resume checklist

- [ ] Confirm `fix-auth0-access-token-email` work is merged / no longer touching auth files.
- [ ] Refresh this worktree's base to current `master`.
- [ ] Add `rust-toolchain.toml` pinning stable (decision A).
- [ ] Add `rustfmt.toml` declaring the drift-prone options.
- [ ] Run one `cargo fmt` sweep; review the diff; commit as an isolated "fmt sweep" commit.
- [ ] Add the `cargo fmt --check` gate (decide: GH Actions workflow vs `bin/` guard vs
      pre-commit) and confirm it passes green.
