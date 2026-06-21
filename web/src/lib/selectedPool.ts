/**
 * Sticky pool selection persisted to localStorage, shared across the
 * Scoreboard / All Tips / Perfect pages.
 *
 * The selection is a three-state value the UI threads as `string | null |
 * undefined`:
 *   - `string`    → a specific pool id.
 *   - `null`      → the explicit "everyone" (global) board.
 *   - `undefined` → not chosen yet → defer to the viewer's first pool.
 *
 * Storage encodes the explicit "everyone" choice as a sentinel so it is
 * distinguishable from "not chosen" (an absent key).
 */
export const SELECTED_POOL_KEY = 'xpool.selectedPool'

/** Stored marker for an explicit "everyone" selection. */
const EVERYONE_SENTINEL = '__everyone__'

/** Read the persisted selection: a pool id, `null` (everyone), or `undefined`. */
export function readSelectedPool(): string | null | undefined {
  let raw: string | null
  try {
    raw = localStorage.getItem(SELECTED_POOL_KEY)
  } catch {
    return undefined
  }
  if (raw === null || raw === '') return undefined
  if (raw === EVERYONE_SENTINEL) return null
  return raw
}

/** Persist a selection: a pool id, or `null` for the explicit "everyone". */
export function writeSelectedPool(poolId: string | null): void {
  try {
    localStorage.setItem(
      SELECTED_POOL_KEY,
      poolId === null ? EVERYONE_SENTINEL : poolId,
    )
  } catch {
    /* ignore — selection is a convenience, not load-bearing state */
  }
}

/**
 * Resolve the selection actually sent to the API. When the user has not
 * chosen (`undefined`), default to their first pool; if they belong to no
 * pool, fall back to `null` (everyone). An explicit `null`/id is honoured.
 */
export function effectiveSelectedPool(
  selected: string | null | undefined,
  poolIds: readonly string[],
): string | null {
  if (selected === undefined) return poolIds[0] ?? null
  return selected
}
