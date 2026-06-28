/**
 * Single source of truth for the e2e stack's HTTP ports — read from the
 * environment, with a static fallback to the legacy fixed ports.
 *
 * Why read-from-env (not allocate-here): `playwright.config.ts` is evaluated
 * BEFORE `globalSetup` AND is re-loaded in every worker process, so the ports
 * must be decided exactly ONCE, before Playwright starts, then handed down via
 * env so every process agrees. The `npm run e2e` wrapper (`e2e/run-e2e.mjs`)
 * allocates two distinct free TCP ports and exports `XPOOL_E2E_WEB_PORT` /
 * `XPOOL_E2E_API_PORT` before spawning Playwright; the main process, its
 * workers, `globalSetup` and `globalTeardown` all inherit them. This lets
 * multiple `npm run e2e` runs execute concurrently without fighting over fixed
 * ports.
 *
 * Running `playwright test` directly (no wrapper) leaves the env unset and falls
 * back to the legacy fixed ports (web :5174 / api :3001), so a single plain run
 * still works exactly as before.
 */
export const DEFAULT_WEB_PORT = 5174
export const DEFAULT_API_PORT = 3001

export interface E2ePorts {
  readonly web: number
  readonly api: number
}

function readPort(value: string | undefined, fallback: number): number {
  if (value === undefined || value === '') return fallback
  const n = Number(value)
  if (!Number.isInteger(n) || n <= 0 || n > 65535) {
    throw new Error(`invalid e2e port "${value}" (expected an integer 1-65535)`)
  }
  return n
}

export function e2ePorts(env: Record<string, string | undefined> = process.env): E2ePorts {
  return {
    web: readPort(env.XPOOL_E2E_WEB_PORT, DEFAULT_WEB_PORT),
    api: readPort(env.XPOOL_E2E_API_PORT, DEFAULT_API_PORT),
  }
}

/**
 * Repo-root-relative path of THIS run's DynamoDB table-name file, written by
 * `scripts/e2e-stack.sh` and namespaced by the (dynamic) API port. Specs that
 * drive xtask against the run's table read it from here; the namespacing keeps
 * concurrent runs from reading each other's table name.
 */
export function e2eTableFile(env: Record<string, string | undefined> = process.env): string {
  return `web/.e2e-table.${e2ePorts(env).api}`
}
