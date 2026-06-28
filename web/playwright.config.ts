import { defineConfig, devices } from '@playwright/test'
import { e2ePorts } from './e2e/ports'

/**
 * Playwright E2E config — runs the suite against an ISOLATED live stack on its
 * OWN ports (DynamoDB Local :8001 + a per-run dynamic axum API port + a per-run
 * dynamic Vite web port), distinct from the dev stack (:8000 / :3000 / :5173).
 * The two coexist: running e2e never hijacks or tears down a running
 * `npm run dev` / `bin/local-dev` session.
 *
 * Ports are DYNAMIC so multiple `npm run e2e` runs can execute concurrently
 * without fighting over fixed ports. `npm run e2e` runs `e2e/run-e2e.mjs`, which
 * allocates two free ports and exports them as XPOOL_E2E_WEB_PORT /
 * XPOOL_E2E_API_PORT before launching Playwright; this config (and every worker
 * that re-loads it) reads them via `e2ePorts()`. Running `playwright test`
 * directly falls back to the legacy fixed ports (:5174 / :3001). See
 * `e2e/ports.ts`.
 *
 * - `globalSetup` boots the backend stack (docker + xtask + API) on the e2e
 *   ports — see `e2e/global-setup.ts` → `scripts/e2e-stack.sh`.
 * - `webServer` starts a dedicated Vite server on the e2e web port (proxying to
 *   the e2e API); `reuseExistingServer: false` so it never adopts dev's :5173.
 * - `globalTeardown` stops the e2e API (docker is left running).
 *
 * Run with `npm run e2e`. Prerequisites: Docker, the Rust toolchain
 * (`rustup`), and `npx playwright install chromium` already done.
 */
const { web: WEB_PORT, api: API_PORT } = e2ePorts()

export default defineConfig({
  testDir: './e2e',
  // Per-run output dir, namespaced by the (dynamic) API port: concurrent runs
  // must not share `test-results/`, or they corrupt each other's traces and
  // attachments (Playwright clears + writes this dir per run).
  outputDir: `./test-results/run-${API_PORT}`,
  // Only `*.spec.ts` are Playwright specs; `*.test.ts` under e2e/ (e.g.
  // ports.test.ts) belong to vitest, so keep Playwright from grabbing them.
  testMatch: '**/*.spec.ts',
  globalSetup: './e2e/global-setup.ts',
  globalTeardown: './e2e/global-teardown.ts',
  // The suite mutates shared backend state (predictions, results), so the
  // specs must run in order, one worker.
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: [['list']],
  timeout: 30_000,
  expect: { timeout: 10_000 },
  use: {
    baseURL: `http://localhost:${WEB_PORT}`,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: {
    // A dedicated Vite server on the e2e web port, proxying to the e2e API.
    command: `npm run dev -- --port ${WEB_PORT} --strictPort`,
    url: `http://localhost:${WEB_PORT}`,
    env: { XPOOL_API_PORT: String(API_PORT) },
    // Never adopt a running dev server — that's the whole point of isolation.
    reuseExistingServer: false,
    timeout: 60_000,
  },
})
