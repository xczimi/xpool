import { useEffect, useState } from 'react'
import {
  composeDisplayMode,
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

/** The two display axes composed into a concrete rendering for the viewport. */
export function useResolvedDisplayMode(): ConcreteDisplayMode {
  const { flag, text } = useDisplayMode()
  const isMobile = useIsMobile()
  return composeDisplayMode(flag, text, isMobile)
}
