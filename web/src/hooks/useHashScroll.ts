import { useEffect } from 'react'
import { hashToId } from '../lib/hashAnchor'

/** Briefly applied to the scrolled-to section (see `.tip-form--anchored`). */
const ANCHOR_CLASS = 'tip-form--anchored'
// Matches the `anchor-pulse` animation duration in index.css (1.4s).
const HIGHLIGHT_MS = 1400

/**
 * Smooth-scroll the element whose id matches `location.hash` into view and
 * pulse it briefly. `contentKey` is a signal that the anchorable content has
 * (re)rendered — e.g. `"R32:10"` (active round + section count) — so the scroll
 * re-runs after async data loads or a round-tab switch, when the target element
 * first exists in the DOM. A `requestAnimationFrame` lets that render commit
 * before we look the element up.
 *
 * The hash is client-side scroll only; react-router still owns the routed round
 * level (`/mytips/:groupId`). No `Date.now()` / clock involvement.
 */
export function useHashScroll(hash: string, contentKey: string): void {
  useEffect(() => {
    const id = hashToId(hash)
    if (!id) return
    let highlightTimer: number | undefined
    const raf = requestAnimationFrame(() => {
      const el = document.getElementById(id)
      if (!el) return
      el.scrollIntoView({ behavior: 'smooth', block: 'start' })
      el.classList.add(ANCHOR_CLASS)
      highlightTimer = window.setTimeout(
        () => el.classList.remove(ANCHOR_CLASS),
        HIGHLIGHT_MS,
      )
    })
    return () => {
      cancelAnimationFrame(raf)
      if (highlightTimer !== undefined) window.clearTimeout(highlightTimer)
    }
  }, [hash, contentKey])
}
