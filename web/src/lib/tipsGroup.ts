/**
 * Persists the last tips group the viewer looked at — a group-node id, i.e. a
 * leaf group ("A".."L", or knockout "KO-M73") or a round-node id ("R16") — so
 * switching between the My Tips and All Tips pages lands on the same group. The
 * stored id is resolved back to a round + group via `resolveGroupParam`.
 */
const KEY = 'xpool.tipsGroup'

export function readTipsGroup(): string | null {
  try {
    return localStorage.getItem(KEY)
  } catch {
    return null
  }
}

export function writeTipsGroup(groupId: string): void {
  try {
    localStorage.setItem(KEY, groupId)
  } catch {
    // Storage may be unavailable (private mode / disabled) — the preference is
    // a convenience, so silently skip rather than break navigation.
  }
}
