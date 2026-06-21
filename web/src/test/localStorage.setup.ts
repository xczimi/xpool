/**
 * Vitest runs in the `node` environment (no DOM, no real `localStorage`).
 * Modules under `src/lib/` persist sticky UI state to `localStorage`; this
 * setup installs a Map-backed `Storage` global so those modules are unit-
 * testable without pulling in a full DOM environment. Mirrors the inline
 * `fakeStorage` stub in `src/auth/pendingInvite.test.ts`.
 */
function mapStorage(): Storage {
  const m = new Map<string, string>()
  return {
    get length() {
      return m.size
    },
    clear: () => m.clear(),
    getItem: (k: string) => (m.has(k) ? (m.get(k) as string) : null),
    key: (i: number) => Array.from(m.keys())[i] ?? null,
    removeItem: (k: string) => {
      m.delete(k)
    },
    setItem: (k: string, v: string) => {
      m.set(k, v)
    },
  }
}

globalThis.localStorage = mapStorage()
