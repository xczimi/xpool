# xpool — Test Strategy & SDLC

How xpool is tested at every layer, how tests stay **isolated**, and how
**time-dependent behaviour** is made deterministic. Authoritative.

Related: [`SCENARIOS.md`](./SCENARIOS.md) (the behaviour catalogue each test
covers), [`API.md`](./API.md), [`DATA_MODEL.md`](./DATA_MODEL.md).

---

## 1. Test layers

| Layer | Lives in | Backed by | Runs |
|---|---|---|---|
| **Domain unit** | `crates/domain/{src,tests}` | nothing — pure functions | always |
| **Crate integration** | `crates/*/tests` | `InMemoryRepository` | always |
| **DynamoDB integration** | `crates/storage/tests/dynamo.rs` | DynamoDB Local | gated `DYNAMO_TEST=1` |
| **API integration** | `crates/api/tests` | `InMemoryRepository` + GraphQL schema | always |
| **End-to-end** | `web/e2e` | the live stack (DynamoDB Local + API + SPA) | `npm run e2e` |

Each layer tests what the ones below it cannot: the domain proves the
**rules**, crate/API integration proves the **wiring**, e2e proves the
**SPA↔API contract on the wire** (the class of bug a `build` never catches).

## 2. Isolation

A test must not see state left by another test or another run.

- **Domain / crate / API layers** — every test constructs a fresh
  `InMemoryRepository`. Isolation is free; nothing to manage.
- **DynamoDB integration** — each test uses a uniquely-named table
  (`xpool-test-<nanos>`), creates it, and drops it on completion.
- **End-to-end** — **one fresh table per run**. `e2e-stack.sh` generates
  `XPOOL_TABLE=xpool-e2e-<timestamp>-<pid>-<rand>` and threads it to `import`,
  `seed`, and the API; `global-teardown` drops it. Within a single run the tests
  share that table, so **each test creates its own uniquely-named data** (unique
  pool names, its own players' predictions) and never asserts on global
  collections like "all pools".

  The e2e stack also runs on its **own ports**, **dynamic per run**: `npm run e2e`
  (`e2e/run-e2e.mjs`) allocates two free TCP ports — one for the API, one for
  Vite — and exports them as `XPOOL_E2E_API_PORT` / `XPOOL_E2E_WEB_PORT`; the
  whole Playwright process tree (config, workers, global setup/teardown, the
  spawned API + Vite) inherits them via `web/e2e/ports.ts`. A bare
  `playwright test` (no wrapper) falls back to the legacy fixed ports
  (`:3001` / `:5174`). DynamoDB is the isolated `:8001` container (the `e2e`
  compose profile), shared but isolated per run by the unique table name. All
  per-run state files (pid/log/table) and Playwright's `outputDir` are namespaced
  by the API port. Playwright sets `reuseExistingServer: false`, and
  `e2e-stack.sh` / `e2e-teardown.sh` only ever kill processes on **this run's**
  dynamic API port (and only pre-emptively on the fixed fallback path). So
  `npm run e2e` and a running `npm run dev` / `bin/local-dev` session **coexist**
  (dev's `:3000` / `:5173` / `:8000` are never touched), and **multiple e2e runs
  can execute concurrently** without colliding. (Ports are threaded via
  `XPOOL_PORT` for the API and `XPOOL_API_PORT` for the Vite proxy; see
  `web/e2e/ports.ts` and `web/playwright.config.ts`.)

**Rule:** DynamoDB Local is in-memory and the container is long-lived — it is
*not* a clean slate. Never rely on an empty database; rely on the isolation
mechanism for your layer.

## 3. The clock — deterministic time

Time-dependent behaviour (deadlines, effective-lock, hidden-until-locked, the
Today window, result-pending polling) is only testable if "now" is
**controllable**. xpool is **server-authoritative**: the API owns the clock,
the SPA never derives time logic from the browser.

### 3.1 Domain — `now` is a parameter

Pure scoring/lock functions take `now: DateTime<Utc>` explicitly
(`effective_locked`, `score_tournament`). They have no clock. Tests pass any
instant. This layer is already correct — do not change it.

### 3.2 API — `now` resolved per request

The edge resolves a single `now` for each request, in priority order:

1. **`X-Dev-Now` header** — RFC3339 instant. The per-request override.
2. **`XPOOL_NOW` env** — RFC3339 instant. A process-wide default.
3. **`Utc::now()`** — real time. Production.

The resolved `now` is placed in the GraphQL context; resolvers and the
post-result recompute read it from there. **No resolver calls `Utc::now()`
directly.**

`X-Dev-Now` and `XPOOL_NOW` are **dev/test stubs** — see [§5](#5-security).

### 3.3 Frontend — server-authoritative, no `Date.now()` for logic

The SPA must not branch on `Date.now()`. The API exposes time-derived **flags**
the SPA simply renders:

- `Group.deadlinePassed` — the node's deadline is in the past.
- `Game.resultPending` — kickoff + an ET-aware buffer has passed and no locked
  official result exists yet (drives smart-polling, `API.md` §7).
- `Game.withinTodayWindow` — kickoff is within ±2 days of `now` (drives the
  Today screen).
- a top-level `now` query field — for "as of" display and the dev clock.

`Date.now()` / `new Date()` is allowed **only for formatting** a timestamp for
display, never for a behavioural decision.

### 3.4 Setting the test clock

- **Domain/crate/API tests** — pass `now` directly, or set the `X-Dev-Now`
  header in the GraphQL request.
- **e2e** — `e2e-stack.sh` sets `XPOOL_NOW` to an instant inside the tournament
  window so the seeded real-2026 fixture is "live". A test that needs a
  different instant uses the **dev clock control** (a datetime picker in the
  auth bar, dev builds only), which sends `X-Dev-Now` on every request — the
  same way the dev player picker sends `X-Dev-Player`.

The fixture (`tournaments/fwc26.json`) keeps **authentic 2026 dates**; the
clock moves, not the data.

## 4. Running the suites

```sh
cargo test --workspace                 # domain + crate + API layers
DYNAMO_TEST=1 cargo test -p storage     # + DynamoDB integration (needs Local)
cargo clippy --workspace -- -D warnings
cd web && npm run build && npm run lint
cd web && npm run e2e                   # boots the whole stack itself
```

A change is "done" only when `cargo test --workspace`, the web `build` +
`lint`, and `npm run e2e` are all green — see
[`verification-before-completion`]. `npm run e2e` is the gate for any
SPA↔API-facing change.

## 5. Security

`X-Dev-Player` and `X-Dev-Now` are **Phase-1 dev stubs** with real exposure:
the first impersonates any player, the second moves the clock — and the clock
governs the write path (deadlines, locking). Honoured unconditionally today
because auth itself is a dev stub (`DATA_MODEL.md` §12). **Both must be gated
off — together — before any real deployment.** They are one dev-stub layer; do
not ship either.

## 6. Open

- Per-*test* e2e isolation (reseed per test) is deferred — per-run + unique
  per-test data is the current bar. Revisit if within-run coupling appears.
- The `X-Dev-Now` dev clock control and the gating of both dev stubs are
  tracked with the real-auth (Auth0) work.
