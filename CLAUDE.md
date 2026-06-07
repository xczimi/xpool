# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

xpool — a soccer score-prediction pool for major tournaments (FIFA World Cup,
UEFA Euro). A **rewrite in progress**: the legacy Google App Engine / Python 2.7
app lives in `archive/` as behavioural ground truth (not deployed, not
maintained); the new implementation is at the repo root.

## Toolchain

Rust is installed natively via `rustup` (rust-lang's recommended installer);
`cargo` is on `PATH` — invoke it directly. Node/npm are on `PATH` too.

```sh
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt
```

## Commands

### Rust workspace

- Test one crate: `cargo test -p domain`
- Test one function: `cargo test -p domain effective_locked`
- DynamoDB integration tests are **gated** behind `DYNAMO_TEST=1` (they need
  DynamoDB Local); without it `cargo test -p storage` skips them and stays green.

### Frontend (`web/`)

- `npm run dev` — Vite dev server (`:5173`, proxies `/api` → `:3000`)
- `npm run build` — `tsc -b && vite build`
- `npm run lint` — eslint
- `npm run e2e` — Playwright suite; **boots the whole live stack itself**
  (docker, import, seed, API) via `e2e/global-setup.ts`. The `e2e/` specs are
  not type-checked by `tsc -b`. The e2e stack runs on its **own ports** (API
  `:3001`, Vite `:5174`, isolated DynamoDB `:8001` via the `e2e` compose
  profile), so it **coexists** with a running `npm run dev` / `bin/local-dev` session
  — it never hijacks or tears those down. See `.specs/TESTING.md` §2.

### Running locally (the full stack)

```sh
docker compose up -d                       # DynamoDB Local (:8000) + MailHog
export DYNAMO_ENDPOINT=http://localhost:8000
cargo run -p xtask -- import tournaments/fwc26.json
cargo run -p xtask -- seed    # result-user + 6 demo players + a pool
cargo run -p api              # :3000
cd web && npm run dev                      # :5173
```

DynamoDB Local runs **in-memory** — re-run `import` + `seed` after any restart.
The API clock is overridable via `XPOOL_NOW` (env) or `X-Dev-Now` (header) for testing
time-dependent behaviour; the e2e suite uses a fresh DynamoDB table per run. See `.specs/TESTING.md`.
Seeded dev players: `result-user` (the admin / official results), `demo-ada`,
`demo-alan`, `demo-grace`, `demo-linus`, `demo-margaret`, `demo-dennis`.

### tmux dev session & worktree switching

`bin/local-dev` is an idempotent, self-healing dev session named `xpool`. It brings
infra (DynamoDB + MailHog) up and seeds the branch's table via `bin/local-stack`,
then lays out four panes: `claude` left; `api` / `web` / `shell` right. Re-running
recreates any pane you closed and restarts a crashed or drifted server without
touching healthy ones. Each pane is tagged with a stable `@role` marker
(`claude`/`api`/`web`/`shell`) shown on its border. See
`docs/superpowers/specs/2026-06-06-unified-tmux-dev-session-design.md`.

`bin/local-dev <worktree>` repoints the running session's `api` + `web` servers at a
git worktree without touching docker/DynamoDB (it's the same idempotent command
that boots the session):

```sh
bin/local-dev scoreboard-design   # → .claude/worktrees/scoreboard-design
bin/local-dev                     # the checkout you're in (main checkout by default)
```

It's **single-stack**: the fixed ports (`:3000` api, `:5173` web, `:8000`
DynamoDB) mean only one worktree's servers run at a time, so a switch stops the
current ones first. It finds the panes by `@role` (not index — indices renumber;
not pane titles — the prompt overwrites them) and stops the old stack by **port +
`C-c`**, so strays on fallback ports die too. Each worktree builds into its **own**
`target/` — `bin/local-dev` deliberately does *not* share `CARGO_TARGET_DIR`, because
a shared target lets cargo serve an `api`/`web` binary built from a different
worktree (same package id), so you'd unknowingly run code that isn't in the
worktree you switched to. The api reads its config (`LOCAL_AUTH_ISSUER`, secrets, etc.) from the main
checkout's `.env` via dotenvy's parent-dir walk-up — a worktree under
`.claude/worktrees/` reaches `$PROJECT/.env`, so the dev-login route works there
with no per-worktree `.env`; `bin/run-api` overrides only `XPOOL_TABLE`.
Each branch now uses its own `xpool-<branch>` table, seeded on first use, so
switching branches no longer risks stale data; `bin/local-dev --reseed` forces a
rebuild after an in-place schema change. DynamoDB is shared in-memory state and
is wiped on container restart — `bin/local-dev` re-seeds the active branch's table
automatically when it finds it empty.

## Architecture

A 5-crate Rust workspace (`crates/*`) plus a React SPA (`web/`).

```
domain ─▶ fwc26 ─┐
   └────▶ storage ┼─▶ api ─▶ web
          xtask ──┘
```

- **`crates/domain`** — pure, I/O-free: the entity model (`model.rs`) and the
  scoring engine (`scoring.rs`). `model.rs` is a **locked contract** other
  crates depend on — changing a type there ripples everywhere.
- **`crates/fwc26`** — FIFA-World-Cup-26-specific logic kept *out* of the
  generic domain: the 495-row Annexe C lookup, third-placed ranking, knockout
  bracket resolution.
- **`crates/storage`** — the `Repository` trait with two adapters:
  `InMemoryRepository` (tests) and `DynamoRepository` (single DynamoDB table).
- **`crates/api`** — axum + async-graphql server. `GET /api/graphql` is the
  GraphiQL playground; `POST /api/graphql` executes. `lambda_http` wrapper
  behind the `lambda` feature.
- **`crates/xtask`** — the `import` / `seed` CLI.
- **`web/`** — React + Vite + TS SPA, urql GraphQL client, en/hu i18n.

### Load-bearing design decisions

These are non-obvious and deliberate — read `.specs/` before changing them:

- **Single-tournament domain, multi-tournament storage.** No `tournament_id`
  appears in domain types; the `Repository` prefixes storage keys with
  `CURRENT_TOURNAMENT_ID`. Don't thread a tournament id through `domain`.
- **Official results are a "result user"** — a `Player` with
  `is_result_user = true` whose predictions *are* the official outcomes.
  Scoring is symmetric `score(A, B)`. Player listings must exclude it.
- **Recursive `GroupGame` tree**; each knockout match is wrapped in its own
  one-match group. This is intentional, not legacy debt.
- **Resolvers do no I/O and no domain logic** — the GraphQL query root loads
  coarse items once, resolvers read from memory and call pure `domain`/`fwc26`
  functions.
- **The scoreboard is materialised** and recomputed wholesale in the
  post-result hook (`crates/api/src/recompute.rs`) when a result is entered.
- **Server-authoritative clock.** The API resolves `now` once per request
  (`X-Dev-Now` header → `XPOOL_NOW` env → real clock) into the GraphQL
  context; resolvers read it from there and never call `Utc::now()`. The SPA
  renders server-derived time flags (`deadlinePassed`, `resultPending`,
  `withinTodayWindow`) and never branches on `Date.now()`. See `.specs/TESTING.md`.
- The SPA's urql client is forced to **POST** (`preferGetMethod: false`) —
  urql's GET default would hit the playground page.

## Source of truth

`.specs/` holds the authoritative design — domain model, scoring, API contract,
FWC26 rules, deployment. `SCENARIOS.md` is the test-linked behaviour catalogue
(supersedes `REWRITE_USE_CASES.md` §3); `TESTING.md` is the test strategy
(layers, isolation, the clock model). When code and an older spec conflict,
`DATA_MODEL.md` and `SCORING.md` win (see their "corrections" sections).
`docs/superpowers/plans/` holds implementation plans. `archive/` is legacy
reference only.

## Conventions

- Branding: display name is **xPool**; the repo/crate identifiers stay lowercase
  `xpool`.
- i18n is first-class (English + Hungarian) in `web/src/i18n/strings.ts`;
  `.specs/LEGACY_I18N.md` is the legacy-wording reference.
- Auth is a **dev stub** — the API resolves the current player from an
  `X-Dev-Player` header; there is no real auth yet (deferred).

## Agent skills

### Issue tracker

Issues and PRDs live as markdown files under `.scratch/<feature>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Canonical names — `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context; domain docs live in `.specs/` (no root `CONTEXT.md` / `docs/adr/`). See `docs/agents/domain.md`.
