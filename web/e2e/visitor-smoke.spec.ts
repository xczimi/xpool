import { test, expect } from '@playwright/test'
import { expectNoErrorView, watchNetwork } from './helpers'

/**
 * Visitor smoke test — visits every public route and asserts the page
 * rendered real content via the live API, with NO error view and NO failed
 * GraphQL responses. This is the test that would have caught both shipped
 * bugs: a schema mismatch produces a GraphQL `errors` array → `ErrorView`;
 * urql sending a query as GET hits the GraphiQL playground → not JSON.
 */

interface PublicRoute {
  path: string
  /** A heading/text that proves the page rendered its real content. */
  expectText: string
}

const PUBLIC_ROUTES: PublicRoute[] = [
  { path: '/', expectText: 'Hi there!' },
  { path: '/today', expectText: 'Today' },
  { path: '/games', expectText: 'Schedule' },
  { path: '/scoreboard', expectText: 'Scoreboard' },
  { path: '/perfect', expectText: 'Perfect Predictions' },
  { path: '/rules', expectText: 'Rules' },
]

for (const route of PUBLIC_ROUTES) {
  test(`public route ${route.path} renders without errors`, async ({
    page,
  }) => {
    const net = watchNetwork(page)
    await page.goto(route.path)

    // The persistent chrome must always be present.
    await expect(page.locator('header.app-header h1')).toHaveText('xPool')

    // The page rendered its real content (not a loading/error placeholder).
    await expect(page.locator('main.content')).toContainText(route.expectText)

    // No error view, no GraphQL failures, no uncaught page errors.
    await expectNoErrorView(page)
    await net.assertNoGraphqlErrors()
    net.assertNoPageErrors()
  })
}

test('footer shows copyright and a link to the GitHub repo', async ({
  page,
}) => {
  await page.goto('/')
  const footer = page.locator('footer.app-footer')
  await expect(footer).toContainText('© xczimi')

  const authorLink = footer.getByRole('link', { name: 'xczimi' })
  await expect(authorLink).toHaveAttribute('href', 'https://xczimi.com/')

  const repoLink = footer.getByRole('link', { name: 'GitHub' })
  await expect(repoLink).toHaveAttribute(
    'href',
    'https://github.com/xczimi/xpool',
  )
})

test('the GraphQL API is reached over POST (urql must not use GET)', async ({
  page,
}) => {
  // Regression guard for the shipped bug: urql's default `within-url-limit`
  // sends short queries as GET, which hits the GraphiQL playground instead of
  // executing. We explicitly assert at least one real POST to /api/graphql.
  let sawGraphqlPost = false
  page.on('request', (req) => {
    if (req.url().includes('/api/graphql') && req.method() === 'POST') {
      sawGraphqlPost = true
    }
  })
  await page.goto('/games')
  await expect(page.locator('main.content')).toContainText('Schedule')
  expect(sawGraphqlPost, 'a POST to /api/graphql must have happened').toBe(
    true,
  )
})
