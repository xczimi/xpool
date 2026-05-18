import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'

/**
 * Playwright globalTeardown — stops the API process started by global-setup.
 * Docker is intentionally left running (DynamoDB Local is in-memory and
 * cheap; reusing it speeds up repeated runs). Run `docker compose down` to
 * stop it.
 */
export default function globalTeardown() {
  const here = dirname(fileURLToPath(import.meta.url))
  const script = resolve(here, '../scripts/e2e-teardown.sh')
  console.log('[global-teardown] stopping the API')
  execFileSync('bash', [script], { stdio: 'inherit' })
}
