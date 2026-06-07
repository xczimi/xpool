# Rename `bin/tmux` to reflect what it actually does

Status: needs-triage
Area: bin / docs

## Idea

Rename `bin/tmux` to something that describes its role — bringing up and
switching between dev environments — rather than naming it after one of the
tools it happens to use.

## Motivation

`bin/tmux` does far more than open a tmux session: it brings infra up (DynamoDB
+ MailHog), seeds the branch's table, lays out the dev panes, *and* repoints the
running stack at a git worktree (`bin/tmux <worktree>`, which absorbed the old
`bin/switch`). tmux is just the layout tool. The name undersells it and reads as
"a tmux helper" rather than "the dev-environment entry point."

## Naming candidates

- `bin/dev-up` — clear "bring the dev stack up", but undersells the
  switch-between-worktrees role.
- `bin/dev` — short, covers both up + switch; reads as "the dev command".
- `bin/dev-stack` — emphasises the whole stack (infra + servers + panes).
- Decide in triage; `bin/dev` or `bin/dev-up` are the front-runners.

## Ripple (rename touches these)

`bin/tmux` is referenced in ~9 places — all must be updated together:

- `CLAUDE.md` (the "tmux dev session & worktree switching" section)
- `README.md`
- `docs/superpowers/specs/2026-06-06-unified-tmux-dev-session-design.md`
- `docs/superpowers/specs/2026-05-18-tmux-restarter-design.md`
- `docs/superpowers/plans/2026-06-06-unified-tmux-dev-session.md`
- sibling scripts: `bin/lib.sh`, `bin/local-stack`, `bin/run-api`
- `web/scripts/e2e-stack.sh`
- the script's own self-references / usage strings inside `bin/tmux`

## Open questions

- Final name? (`bin/dev` vs `bin/dev-up` vs `bin/dev-stack`)
- Keep a thin `bin/tmux` shim that forwards to the new name (muscle memory,
  existing docs links), or hard-rename and update everything at once?
- Does the session name (`xpool`) or any `@role` markers reference "tmux" in a
  way that should change too?

## Related

- The memory note [[bin-tmux-targets-invoking-checkout]] documents a behaviour
  to preserve through any rename.
