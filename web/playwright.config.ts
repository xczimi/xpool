import { defineConfig, devices } from '@playwright/test'

/**
 * Playwright E2E config — runs the suite against the FULL live stack
 * (DynamoDB Local + the axum API on :3000 + the Vite dev server on :5173).
 *
 * - `globalSetup` boots the backend stack (docker + xtask + API) — see
 *   `e2e/global-setup.ts` → `scripts/e2e-stack.sh`.
 * - `webServer` starts the Vite dev server; Playwright waits for it.
 * - `globalTeardown` stops the API (docker is left running).
 *
 * Run with `npm run e2e`. Prerequisites: Docker, the Rust toolchain via
 * `mise`, and `npx playwright install chromium` already done.
 */
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
    baseURL: 'http://localhost:5173',
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
    command: 'npm run dev',
    url: 'http://localhost:5173',
    reuseExistingServer: true,
    timeout: 60_000,
  },
})
