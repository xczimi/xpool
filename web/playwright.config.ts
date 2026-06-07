import { defineConfig, devices } from '@playwright/test'

/**
 * Playwright E2E config — runs the suite against an ISOLATED live stack on its
 * OWN ports (DynamoDB Local :8001 + the axum API :3001 + a Vite dev server
 * :5174), distinct from the dev stack (:8000 / :3000 / :5173). The two coexist:
 * running e2e never hijacks or tears down a running `npm run dev` / `bin/local-dev`
 * session.
 *
 * - `globalSetup` boots the backend stack (docker + xtask + API) on the e2e
 *   ports — see `e2e/global-setup.ts` → `scripts/e2e-stack.sh`.
 * - `webServer` starts a dedicated Vite server on :5174 (proxying to the e2e
 *   API on :3001); `reuseExistingServer: false` so it never adopts dev's :5173.
 * - `globalTeardown` stops the e2e API (docker is left running).
 *
 * Run with `npm run e2e`. Prerequisites: Docker, the Rust toolchain
 * (`rustup`), and `npx playwright install chromium` already done.
 */
// Dedicated e2e ports — kept distinct from the dev stack so the two coexist.
const WEB_PORT = 5174
const API_PORT = 3001

export default defineConfig({
  testDir: './e2e',
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
