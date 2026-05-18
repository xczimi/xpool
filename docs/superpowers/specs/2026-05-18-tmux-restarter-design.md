# tmux dev-session restarter — design

**Date:** 2026-05-18
**Status:** approved

## Goal

A single script, `bin/tmux`, that brings up the full xpool local dev stack in a
tmux session and re-attaches to it on subsequent runs. Modelled on
`../../Southsiders/membership-new/bin/tmux`, adapted to xpool's ordering
constraints.

## Behavior

1. **Re-attach shortcut.** If a tmux session named `xpool` already exists,
   `tmux -CC attach` to it and exit. No data or containers are touched.
2. **Bootstrap (fresh session only).** Runs *before* any panes are created so
   ordering is guaranteed:
   - `docker compose up -d` — DynamoDB Local (:8000) + MailHog
   - poll `nc -z localhost 8000` until DynamoDB accepts connections
     (the container has no healthcheck)
   - `cargo run -p xtask -- import tournaments/fwc26.json`
   - `cargo run -p xtask -- seed`
3. **Create panes**, then attach with `tmux -CC` (iTerm2 control mode).

DynamoDB Local runs in-memory, so import + seed must re-run after any container
restart — the bootstrap always runs on a fresh session for that reason.

## Layout

```
┌───────────┬─────────────┐
│           │ docker logs │   pane 1: docker compose logs -f
│           ├─────────────┤
│  claude   │ cargo api   │   pane 2: cargo run -p api
│  (pane 0) ├─────────────┤
│           │ web npm dev │   pane 3: cd web && npm run dev
└───────────┴─────────────┘
```

- **Pane 0 (left):** launches `claude` — the primary interactive session,
  focused on attach.
- **Right column:** three stacked panes — docker logs, API, web dev server.

## Details

- `SESSION="xpool"`; project directory hardcoded to `~/Private/SoccerPool/xpool`,
  matching the membership-new convention.
- `DYNAMO_ENDPOINT=http://localhost:8000` is exported for the bootstrap steps.
  tmux panes do not inherit the script's environment, so the API pane receives
  it inline on its `send-keys` command.
- Right-column panes are targeted by explicit pane index for deterministic
  layout; `select-layout` evens out the three right panes.
- The legacy `archive/` GAE app is untouched — this only drives the new stack.

## Trade-off accepted

The bootstrap blocks the terminal while `cargo run -p xtask` compiles (slow on a
cold `target/`). Bootstrapping inside a pane would lose the strict ordering
guarantee, so blocking is accepted; the output is visible and it is a one-time
cost per fresh session.
