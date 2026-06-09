/**
 * Which deployment the SPA is running against, derived purely from the
 * browser hostname (frontend-only, no API call). `prod` is the default —
 * absence of a tag means production. Used to suffix the header wordmark so
 * the environment is obvious without reading the URL bar.
 */
export type AppEnv = 'local' | 'dev' | 'prod'

export function detectEnv(
  hostname: string = window.location.hostname,
): AppEnv {
  const h = hostname.toLowerCase()
  if (h === 'localhost' || h === '127.0.0.1' || h.includes('local')) {
    return 'local'
  }
  if (h.includes('dev')) {
    return 'dev'
  }
  return 'prod'
}

/**
 * The wordmark suffix for an environment, e.g. `·dev`. Returns null for
 * production (no suffix shown).
 */
export function envSuffix(env: AppEnv): string | null {
  return env === 'prod' ? null : `·${env}`
}
