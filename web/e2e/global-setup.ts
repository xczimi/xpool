import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'
import { e2ePorts } from './ports'

/**
 * Playwright globalSetup — boots the full backend stack before any test runs.
 * Delegates to `scripts/e2e-stack.sh`, which is idempotent: it kills stale
 * processes, brings docker up, imports + seeds the tournament, and starts the
 * API, then waits for `GET /api/health`.
 *
 * The per-run ports (decided once by `e2e/run-e2e.mjs`, or the fixed fallback
 * for a bare `playwright test`) are passed explicitly to the stack script so it
 * binds the API to the same port this run's webServer/baseURL target.
 */
export default function globalSetup() {
  const here = dirname(fileURLToPath(import.meta.url))
  const script = resolve(here, '../scripts/e2e-stack.sh')
  const { web, api } = e2ePorts()
  console.log(
    `[global-setup] booting the live stack (api=:${api}, web=:${web}) via scripts/e2e-stack.sh`,
  )
  execFileSync('bash', [script], {
    stdio: 'inherit',
    env: {
      ...process.env,
      XPOOL_E2E_API_PORT: String(api),
      XPOOL_E2E_WEB_PORT: String(web),
    },
  })
}
