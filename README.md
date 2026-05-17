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

Prerequisites: [`mise`](https://mise.jdx.dev/) (provides Rust), Node 20+, Docker.

```sh
# 1. Start DynamoDB Local + MailHog
docker compose up -d

# 2. Import the tournament and seed demo data
export DYNAMO_ENDPOINT=http://localhost:8000
mise exec -- cargo run -p xtask -- import tournaments/fwc26.json
mise exec -- cargo run -p xtask -- seed

# 3. Run the API (http://localhost:3000)
mise exec -- cargo run -p api

# 4. In another terminal, run the SPA (http://localhost:5173)
cd web && npm install && npm run dev
```

The SPA proxies `/api` to the server. DynamoDB Local runs in-memory — after a
container restart, re-run the `import` and `seed` steps.

**Dev login:** auth is a stub — the SPA sends an `X-Dev-Player` header. Pick a
seeded player in the SPA's auth bar. Seeded ids: `result-user` (the admin /
official results), and `demo-ada`, `demo-alan`, `demo-grace`, `demo-linus`,
`demo-margaret`, `demo-dennis`. There is one demo pool, `pool-demo`.

**Tests:** `mise exec -- cargo test` (workspace) and `cd web && npm run build`.
DynamoDB integration tests are gated behind `DYNAMO_TEST=1`.

The implementation plan is in
[`docs/superpowers/plans/`](./docs/superpowers/plans/).

## Documentation

Specs and reference docs for agentic development live in [`.specs/`](./.specs/).

| Document | What it covers |
|----------|----------------|
| [`.specs/REWRITE_USE_CASES.md`](./.specs/REWRITE_USE_CASES.md) | User journeys, scenarios, and use cases — *what the app does*, technology-independent |
| [`.specs/REWRITE_IMPLEMENTATION.md`](./.specs/REWRITE_IMPLEMENTATION.md) | Domain model, scoring engine, data ingestion, legacy anti-patterns — *how to build it* |
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
