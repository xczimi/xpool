import { defineConfig } from 'vitest/config'

// Unit-test runner for the SPA's pure logic layer (`src/lib/`).
// Kept separate from `vite.config.ts` so the production build config and
// the test config evolve independently.
export default defineConfig({
  test: {
    environment: 'node',
    setupFiles: ['src/test/localStorage.setup.ts'],
    // `src/**` is the SPA logic layer; `e2e/**` adds the e2e infra unit tests
    // (e.g. the dynamic-port reader). Playwright only ever runs `*.spec.ts`
    // (see playwright.config.ts `testMatch`), so `*.test.ts` under e2e/ belong
    // to vitest alone.
    include: ['src/**/*.test.ts', 'e2e/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      // The `src/lib/` pure-logic modules. `usePolledQuery.ts` is a React
      // hook (effect-driven, not pure) — out of scope for unit coverage.
      include: ['src/lib/**/*.ts'],
      exclude: ['src/lib/**/*.test.ts', 'src/lib/usePolledQuery.ts'],
      reporter: ['text', 'html'],
      thresholds: {
        lines: 80,
        functions: 80,
        branches: 80,
        statements: 80,
      },
    },
  },
})
