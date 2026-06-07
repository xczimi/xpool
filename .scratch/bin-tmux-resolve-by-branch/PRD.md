# `bin/tmux` should resolve a target by branch name too

Status: needs-triage
Area: bin

## Idea

Let `bin/tmux <arg>` find the right worktree when `<arg>` is a **branch name**,
not only when it's the literal worktree **directory name**.

## Motivation

Today `bin/tmux <arg>` appends the arg as a directory:
`TARGET="$PROJECT/.claude/worktrees/$ARG"` (`bin/tmux:39`). If the worktree's
directory name differs from its branch — which happens — passing the branch
fails confusingly:

```
$ bin/tmux worktree-unified-result-entry
bin/tmux: not an xpool checkout (no web/): .../.claude/worktrees/worktree-unified-result-entry
```

…even though that branch *is* checked out, in a worktree directory named
`unified-result-entry`. The user naturally reaches for the branch name (it's
what `git`/`gh` show), hits a dead end, and has to discover the dir name.

## Sketch

- When `.claude/worktrees/<arg>` doesn't exist, fall back to resolving `<arg>`
  as a branch: `git worktree list --porcelain` → find the worktree whose branch
  is `<arg>` (try both `<arg>` and `refs/heads/<arg>`) → use its path.
- Keep the current behaviours unchanged: bare dir name, and an explicit path
  still work; the branch lookup is only a fallback.
- If a branch resolves to no worktree (or more than one), print a clear message
  listing candidates — the script already prints the worktree list on failure,
  so lean on that.

## Open questions

- Precedence when an arg is *both* a valid dir name and a branch name — prefer
  the directory (current behaviour) and only fall back to branch lookup? (likely
  yes)
- Should it offer to *create* the worktree if the branch exists but isn't
  checked out anywhere, or stay strictly a resolver?

## Related

- [[rename-bin-tmux]] — same theme (make `bin/tmux` clearer about what it
  operates on); could land together.
- The naming drift itself (worktree dir `unified-result-entry` vs branch
  `worktree-unified-result-entry`) is the trigger.
