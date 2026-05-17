# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

xpool — a soccer score-prediction pool for major tournaments (FIFA World Cup,
UEFA Euro). A **rewrite in progress**: the legacy Google App Engine / Python 2.7
app lives in `archive/` as behavioural ground truth (not deployed, not
maintained); the new implementation is at the repo root.

## Toolchain

Rust is installed via `mise` (`mise.toml` pins `rust = "latest"`); `cargo` is
on `PATH` — invoke it directly. Node/npm are on `PATH` too.

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
  not type-checked by `tsc -b`.

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
Seeded dev players: `result-user` (the admin / official results), `demo-ada`,
`demo-alan`, `demo-grace`, `demo-linus`, `demo-margaret`, `demo-dennis`.

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
- The SPA's urql client is forced to **POST** (`preferGetMethod: false`) —
  urql's GET default would hit the playground page.

## Source of truth

`.specs/` holds the authoritative design — domain model, scoring, API contract,
FWC26 rules, deployment. When code and an older spec conflict, `DATA_MODEL.md`
and `SCORING.md` win (see their "corrections" sections). `docs/superpowers/plans/`
holds implementation plans. `archive/` is legacy reference only.

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
