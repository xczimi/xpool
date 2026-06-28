import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'
import { e2ePorts } from './ports'

/**
 * Playwright globalTeardown — stops the API process started by global-setup.
 * Docker is intentionally left running (DynamoDB Local is in-memory and
 * cheap; reusing it speeds up repeated runs). Run `docker compose down` to
 * stop it.
 *
 * The API port is passed explicitly so teardown stops THIS run's API (and drops
 * THIS run's table) — never another concurrent run's.
 */
export default function globalTeardown() {
  const here = dirname(fileURLToPath(import.meta.url))
  const script = resolve(here, '../scripts/e2e-teardown.sh')
  const { api } = e2ePorts()
  console.log(`[global-teardown] stopping the API on :${api}`)
  execFileSync('bash', [script], {
    stdio: 'inherit',
    env: { ...process.env, XPOOL_E2E_API_PORT: String(api) },
  })
}
