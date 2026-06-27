# xpool

A soccer score-prediction pool for major tournaments (FIFA World Cup, UEFA
Euro). Players predict match scores and compete on a points scoreboard.

> **Status: rewrite in progress.** The original app (Google App Engine,
> Python 2.7) has been moved to [`archive/`](./archive/) for reference. The new
> implementation lives at the repo root — see [Running locally](#running-locally).

## Architecture

A Rust workspace plus a React SPA, all runnable locally:

| Crate / dir | Role |
|---|---|
| `crates/domain` | Pure entities + the scoring engine (no I/O) |
| `crates/fwc26` | FIFA World Cup 26 logic — Annexe C, bracket resolution |
| `crates/storage` | `Repository` trait — in-memory fake + DynamoDB adapter |
| `crates/api` | axum + async-graphql server (`/api/graphql`) |
| `crates/xtask` | Import / seed CLI |
| `web/` | React + Vite + TypeScript SPA (urql GraphQL client) |
| `tournaments/fwc26.json` | The FWC26 tournament definition (104 matches) |

## Running locally

Prerequisites: the Rust toolchain ([`rustup`](https://rustup.rs/)), Node 20+, Docker.

```sh
# 1. Start DynamoDB Local + MailHog
docker compose up -d

# 2. Import the tournament and seed demo data
export DYNAMO_ENDPOINT=http://localhost:8000
cargo run -p xtask -- import tournaments/fwc26.json
cargo run -p xtask -- seed

# 3. Run the API (http://localhost:3000)
cargo run -p api

# 4. In another terminal, run the SPA (http://localhost:5173)
cd web && npm install && npm run dev
```

The SPA proxies `/api` to the server. DynamoDB Local runs in-memory — after a
container restart, re-run the `import` and `seed` steps.

`bin/xtask <args>` wraps these `xtask` invocations against the current branch's
table (e.g. `bin/xtask import tournaments/fwc26.json`, `bin/xtask seed`).
`bin/deploy [dev|prod]` ships code/infra and `bin/deploy-data` / `bin/pull-data`
move data — see [`.specs/DEPLOYMENT.md`](./.specs/DEPLOYMENT.md) §10.

**One-command session:** `bin/local-dev` is an idempotent, self-healing dev session.
It brings infra + the per-branch DynamoDB table up, lays out a tmux session
(`claude` / `api` / `web` / `shell` panes), and re-running it recreates any pane
you closed and restarts a crashed server — without touching healthy ones. To
point the api + web servers at a git worktree, run `bin/local-dev <worktree>` (or
`bin/local-dev` for the checkout you're in) — only one checkout's stack runs at a
time, since the ports are fixed. See
[`docs/superpowers/specs/2026-06-06-unified-tmux-dev-session-design.md`](./docs/superpowers/specs/2026-06-06-unified-tmux-dev-session-design.md).

**Dev login:** auth is a Bearer-JWT seam — a local issuer for dev, Auth0 for the
deployed stages (real Auth0 sign-in is still deferred). In local dev the SPA's
auth bar lets you pick a seeded player and mints a local-issuer JWT via
`POST /api/dev/login` (the `X-Dev-Player` header is gone). Seeded ids:
`result-user` (the admin / official results), and `demo-ada`, `demo-alan`,
`demo-grace`, `demo-linus`, `demo-margaret`, `demo-dennis`. There is one demo
pool, `pool-demo`.

**Tests:** `cargo test` (workspace), `cd web && npm run build`, `npm run lint`,
`npm run test` (Vitest), and `npm run e2e` (Playwright — boots the full live
stack itself on isolated ports). DynamoDB integration tests are gated behind
`DYNAMO_TEST=1`. See [`.specs/TESTING.md`](./.specs/TESTING.md).

The implementation plan is in
[`docs/superpowers/plans/`](./docs/superpowers/plans/).

## Documentation

Specs and reference docs for agentic development live in [`.specs/`](./.specs/).

| Document | What it covers |
|----------|----------------|
| [`.specs/REWRITE_USE_CASES.md`](./.specs/REWRITE_USE_CASES.md) | User journeys, scenarios, and use cases — *what the app does*, technology-independent |
| [`.specs/REWRITE_IMPLEMENTATION.md`](./.specs/REWRITE_IMPLEMENTATION.md) | Domain model, scoring engine, data ingestion, legacy anti-patterns — *how to build it* |
| [`.specs/SCENARIOS.md`](./.specs/SCENARIOS.md) | Test-linked behaviour catalogue — concrete scenarios mapped to tests (supersedes `REWRITE_USE_CASES.md` §3) |
| [`.specs/TESTING.md`](./.specs/TESTING.md) | Test strategy — layers, isolation, and the server-authoritative clock model |
| [`.specs/DATA_MODEL.md`](./.specs/DATA_MODEL.md) | The agreed domain & storage model — entities, tournament tree, pools, identity, DynamoDB layout |
| [`.specs/SCORING.md`](./.specs/SCORING.md) | The agreed scoring engine — per-match points, standings bonus, multipliers, materialized scoreboard |
| [`.specs/API.md`](./.specs/API.md) | The agreed API & frontend contract — GraphQL, coarse queries/mutations, draft→locked, smart polling |
| [`.specs/DEPLOYMENT.md`](./.specs/DEPLOYMENT.md) | The agreed deployment & infrastructure — environments, OpenTofu, CI/CD, cost posture |
| [`.specs/DESIGN_REVIEW.md`](./.specs/DESIGN_REVIEW.md) | The design-review record — Peter's decisions, rationale, and the calls that overrode recommendations |
| [`.specs/GAME_RULES.md`](./.specs/GAME_RULES.md) | The prediction/scoring rules in detail, including known bugs |
| [`.specs/FWC26_RULES.md`](./.specs/FWC26_RULES.md) | FIFA World Cup 26 competition rules — tournament structure, tiebreakers, knockout bracket |
| [`.specs/DATA_SOURCES.md`](./.specs/DATA_SOURCES.md) | Tournament data sources — FotMob calendar feed, TheSportsDB, and the ingestion flow |
| [`.specs/THESPORTSDB_API.md`](./.specs/THESPORTSDB_API.md) | TheSportsDB API reference — endpoints and the World Cup 26 ingestion subset |
| [`.specs/LEGACY_I18N.md`](./.specs/LEGACY_I18N.md) | Legacy UI strings (English/Hungarian) extracted from the old app — i18n reconciliation reference |

## `archive/`

The complete legacy Google App Engine application, kept as the behavioral
ground truth to consult while rewriting. Not deployed, not maintained.
