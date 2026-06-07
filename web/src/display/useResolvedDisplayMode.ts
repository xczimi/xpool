import { useEffect, useState } from 'react'
import {
  resolveDisplayMode,
  type ConcreteDisplayMode,
} from '../lib/displayMode'
import { useDisplayMode } from './useDisplayMode'

// Matches the SPA's mobile breakpoint in index.css (640px) so `auto` flips to
// flag-only exactly when the layout goes mobile.
const MOBILE_QUERY = '(max-width: 640px)'

/** Track the mobile media query, updating live on resize/rotate. */
function useIsMobile(): boolean {
  const [isMobile, setIsMobile] = useState(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return false
    return window.matchMedia(MOBILE_QUERY).matches
  })

  useEffect(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return
    const mql = window.matchMedia(MOBILE_QUERY)
    const onChange = (e: MediaQueryListEvent) => setIsMobile(e.matches)
    mql.addEventListener('change', onChange)
    return () => mql.removeEventListener('change', onChange)
  }, [])

  return isMobile
}

/** The current display mode with `auto` resolved against the viewport. */
export function useResolvedDisplayMode(): ConcreteDisplayMode {
  const { mode } = useDisplayMode()
  const isMobile = useIsMobile()
  return resolveDisplayMode(mode, isMobile)
}
