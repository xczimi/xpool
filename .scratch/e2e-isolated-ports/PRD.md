# Smarter e2e — isolate it from the local dev stack (own ports)

Status: needs-triage
Area: web (e2e) / bin

## Idea

Make the e2e suite run on its **own ports** (and its own stack) so it doesn't
collide with — or clobber — a running local dev session, and make its
stack-boot smarter overall.

## Motivation

Today e2e shares the dev stack's fixed ports and even reuses dev's servers:

- `web/playwright.config.ts`: `baseURL: http://localhost:5173`,
  `webServer.url: http://localhost:5173`, **`reuseExistingServer: true`** — so
  if your dev Vite server is up, Playwright *hijacks it*.
- API on `:3000`, DynamoDB on `:8000` — the same ports as `npm run dev` /
  `bin/local-dev` (`:3000` api, `:5173` web, `:8000` DynamoDB).
- `web/scripts/e2e-stack.sh` "kills stale processes" on boot — so running e2e
  **kills your dev API**, and the suite then mutates shared backend state
  (`fullyParallel: false`, `workers: 1`, because state is shared).

Net effect: you can't run e2e while developing without it tearing down your dev
servers and potentially corrupting the data you were looking at. The two should
be able to coexist.

## Sketch

- **Dedicated e2e port range**, distinct from dev — e.g. API `:3001`, web
  `:5174`, DynamoDB `:8001` (exact numbers TBD). Thread these through
  `e2e-stack.sh`, the API/web boot, and `playwright.config.ts`
  (`baseURL` / `webServer.url`).
- **Never reuse the dev server**: set `reuseExistingServer: false` for the e2e
  web server (or point it at the e2e-only port so reuse can't hit dev).
- **Don't kill dev processes**: `e2e-stack.sh` should only tear down *its own*
  e2e-port processes, not whatever's on the dev ports.
- **Isolated DynamoDB**: already a fresh table per run — keep that; consider a
  separate DynamoDB Local container/port so the e2e table can't touch dev's
  in-memory data.

## "Smarter" extras (optional, triage which are worth it)

- Detect a running dev stack and log "running e2e on isolated ports, dev stack
  left untouched" instead of silently fighting over ports.
- Faster boot: reuse the e2e container across runs when schema is unchanged
  (the `--reseed`-style escape hatch from `bin/local-dev`).
- Revisit whether isolated state per worker could let e2e go parallel
  (`workers > 1`) — currently serial only because state is shared.

## Open questions

- Fixed e2e port range, or dynamically allocate free ports per run (more robust
  on CI / multiple checkouts, but harder to attach a browser to)?
- Separate DynamoDB Local container vs. just a separate table on the shared
  `:8000` instance?
- Should `bin/local-dev` learn an "e2e" awareness so the two stacks are visibly
  distinct?

## Related

- [[rename-bin-tmux]] / [[bin-tmux-resolve-by-branch]] — same dev-tooling
  surface; the dev stack's fixed single-port design is what e2e collides with.
- The memory note "Frontend work needs E2E" — making e2e painless to run
  alongside dev directly supports that.
