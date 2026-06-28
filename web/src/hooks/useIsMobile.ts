import { useEffect, useState } from 'react'

// Matches the SPA's mobile breakpoint in index.css (640px) so layout-mode
// branches flip exactly when the CSS goes mobile.
export const MOBILE_QUERY = '(max-width: 640px)'

/** Track the mobile media query, updating live on resize / rotate. */
export function useIsMobile(): boolean {
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
