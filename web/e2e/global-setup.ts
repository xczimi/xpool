import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'

/**
 * Playwright globalSetup — boots the full backend stack before any test runs.
 * Delegates to `scripts/e2e-stack.sh`, which is idempotent: it kills stale
 * processes, brings docker up, imports + seeds the tournament, and starts the
 * API, then waits for `GET /api/health`.
 */
export default function globalSetup() {
  const here = dirname(fileURLToPath(import.meta.url))
  const script = resolve(here, '../scripts/e2e-stack.sh')
  console.log('[global-setup] booting the live stack via scripts/e2e-stack.sh')
  execFileSync('bash', [script], { stdio: 'inherit' })
}
