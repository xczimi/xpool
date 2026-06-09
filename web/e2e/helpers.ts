import { expect, type Page } from '@playwright/test'

/**
 * Shared E2E helpers. The recurring theme: the two bugs that shipped were an
 * integration mismatch between the SPA and the API (a GraphQL schema mismatch,
 * and urql sending queries as GET). A `build`-only check cannot catch those —
 * only exercising the real wire can. These helpers make every test assert on
 * the actual GraphQL traffic and the rendered result.
 */

/** Text the `ErrorView` component renders on a GraphQL/network failure. */
export const ERROR_TEXT = 'Something went wrong'

interface NetworkWatch {
  /** Throws if any GraphQL response carried an `errors` array or non-200. */
  assertNoGraphqlErrors: () => Promise<void>
  /** Throws if the page logged an uncaught error / pageerror. */
  assertNoPageErrors: () => void
}

/**
 * Attach listeners that record uncaught page errors and failed GraphQL
 * responses. Call BEFORE navigating. The GraphQL check inspects both the HTTP
 * status and the response body's `errors` field — a query sent as GET would
 * hit the GraphiQL playground (HTML, not JSON) and be flagged here.
 */
export function watchNetwork(page: Page): NetworkWatch {
  const pageErrors: string[] = []
  const graphqlErrors: string[] = []

  page.on('pageerror', (err) => {
    pageErrors.push(err.message)
  })

  page.on('response', async (res) => {
    const url = res.url()
    if (!url.includes('/api/graphql')) return
    const method = res.request().method()
    // Ignore CORS preflight — only the actual GraphQL call matters.
    if (method === 'OPTIONS') return
    if (method !== 'POST') {
      graphqlErrors.push(
        `GraphQL request used ${method} (must be POST): ${url}`,
      )
      return
    }
    if (!res.ok()) {
      graphqlErrors.push(`GraphQL HTTP ${res.status()} on ${url}`)
      return
    }
    // Read the body as text once. An unreadable body (served from cache, or
    // the request was aborted by a client teardown) is not a server error, so
    // it is ignored. HTML means the query hit the GraphiQL playground — the
    // exact symptom of urql sending the query as GET.
    let text: string
    try {
      text = await res.text()
    } catch {
      return
    }
    if (text.trim() === '') return
    if (/^\s*</.test(text)) {
      graphqlErrors.push(
        `GraphQL response was HTML, not JSON (query sent as GET?) on ${url}`,
      )
      return
    }
    try {
      const body = JSON.parse(text)
      if (body && Array.isArray(body.errors) && body.errors.length > 0) {
        graphqlErrors.push(
          `GraphQL errors on ${url}: ${JSON.stringify(body.errors)}`,
        )
      }
    } catch {
      graphqlErrors.push(`GraphQL response was not JSON on ${url}: ${text.slice(0, 120)}`)
    }
  })

  return {
    assertNoGraphqlErrors: async () => {
      // Give in-flight responses a beat to settle.
      await page.waitForTimeout(200)
      expect(graphqlErrors, graphqlErrors.join('\n')).toEqual([])
    },
    assertNoPageErrors: () => {
      expect(pageErrors, pageErrors.join('\n')).toEqual([])
    },
  }
}

/** Assert the page rendered real content and shows no `ErrorView`. */
export async function expectNoErrorView(page: Page): Promise<void> {
  await expect(page.locator('.status-error')).toHaveCount(0)
  await expect(page.getByText(ERROR_TEXT)).toHaveCount(0)
}

/** Log out via the auth bar, if currently logged in. */
export async function devLogout(page: Page): Promise<void> {
  // Direct child only — the logout button. A nested `.dev-clock` reset button
  // also exists when a dev clock is pinned, so `.auth-bar button` is ambiguous.
  const logoutBtn = page.locator('.auth-bar > button')
  if (await logoutBtn.isVisible().catch(() => false)) {
    await logoutBtn.click()
  }
  await expect(page.locator('.auth-bar')).toContainText('You are outside.')
}

/**
 * Open the header settings popover (gear), where the language / display / theme
 * controls now live. Idempotent: only clicks the gear if the panel is closed,
 * so it is safe to call when a previous step already opened it. The gear's
 * accessible name is localised, so we target it by class, not by role+name.
 */
export async function openSettings(page: Page): Promise<void> {
  const gear = page.locator('.settings-gear')
  await expect(gear).toBeVisible()
  if ((await gear.getAttribute('aria-expanded')) !== 'true') {
    await gear.click()
  }
  await expect(page.getByRole('dialog')).toBeVisible()
}

/**
 * Dev-login as a seeded player via the auth bar `<select>`. Logs out first if
 * already logged in (the picker is hidden while logged in), then asserts the
 * auth bar shows the chosen player as logged in.
 */
export async function devLogin(page: Page, playerId: string): Promise<void> {
  await devLogout(page)
  const select = page.locator('.auth-picker select')
  await expect(select).toBeVisible()
  // Wait for the players query to populate the picker.
  await expect(select.locator(`option[value="${playerId}"]`)).toHaveCount(1)
  await select.selectOption(playerId)
  await expect(page.locator('.auth-bar')).toContainText('Logged in as')
}
