/**
 * Pull an invite code out of either a full link (`…/invite/<code>`) or a bare
 * code pasted on its own. Returns null when nothing usable is found. Used by
 * the invite dead-end (`NeedsInvite`) to turn whatever a viewer pastes into a
 * route to the public claim page.
 */
export function extractCode(raw: string): string | null {
  const value = raw.trim()
  if (!value) return null
  const marker = '/invite/'
  const at = value.indexOf(marker)
  const code = at >= 0 ? value.slice(at + marker.length) : value
  // Drop any trailing path/query/hash and surrounding slashes.
  const cleaned = code.split(/[/?#]/)[0].trim()
  return cleaned || null
}
