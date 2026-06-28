/**
 * Normalise a `location.hash` (`#KO-M76`) to a DOM element id (`KO-M76`).
 * Returns `''` for an empty/bare hash so callers can early-return. The id is a
 * stable `group.id` (group-stage `A`..`L` or knockout `KO-M73`), so a percent-
 * encoded hash is decoded; an undecodable hash falls back to its raw form.
 */
export function hashToId(hash: string): string {
  const raw = hash.replace(/^#/, '')
  if (!raw) return ''
  try {
    return decodeURIComponent(raw)
  } catch {
    return raw
  }
}
