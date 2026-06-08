import { useEffect, useMemo, useState, type ReactNode } from 'react'
import {
  ACCENT_STORAGE_KEY,
  MODE_STORAGE_KEY,
  coerceAccent,
  coerceThemeMode,
  resolveTheme,
  type Accent,
  type ThemeMode,
} from './theme'
import { ThemeContext, type ThemeState } from './themeContextValue'

const DARK_QUERY = '(prefers-color-scheme: dark)'

function getSystemPrefersDark(): boolean {
  return window.matchMedia(DARK_QUERY).matches
}

function readStored<T>(key: string, coerce: (v: unknown) => T): T {
  try {
    return coerce(localStorage.getItem(key))
  } catch {
    return coerce(null)
  }
}

function write(key: string, value: string): void {
  try {
    localStorage.setItem(key, value)
  } catch {
    /* ignore */
  }
}

/** Accent + dark/light theme preference — persisted to localStorage. */
export function ThemeProvider({ children }: { children: ReactNode }) {
  const [accent, setAccentState] = useState<Accent>(() =>
    readStored(ACCENT_STORAGE_KEY, coerceAccent),
  )
  const [mode, setModeState] = useState<ThemeMode>(() =>
    readStored(MODE_STORAGE_KEY, coerceThemeMode),
  )
  // Tracks the OS dark-mode preference; updated only by the matchMedia
  // change-event listener so there is no synchronous setState in an effect.
  // We always read mql.matches at render time instead of relying on stale
  // state, so a mode switch back to 'system' reflects the current OS setting.
  const [osChangeCount, setOsChangeCount] = useState(0)

  // Subscribe to OS colour-scheme changes while mode=system.
  useEffect(() => {
    if (mode !== 'system') return
    const mql = window.matchMedia(DARK_QUERY)
    const onChange = () => setOsChangeCount((n) => n + 1)
    mql.addEventListener('change', onChange)
    return () => mql.removeEventListener('change', onChange)
  }, [mode])

  // Read the current OS preference synchronously at render time.
  // osChangeCount is consumed here so the component re-renders on OS changes.
  void osChangeCount
  const prefersDark = getSystemPrefersDark()

  const resolved = resolveTheme(mode, prefersDark)

  // Mirror the resolved theme + accent onto <html> for the CSS to read.
  useEffect(() => {
    const root = document.documentElement
    root.setAttribute('data-accent', accent)
    root.setAttribute('data-theme', resolved)
  }, [accent, resolved])

  const value = useMemo<ThemeState>(
    () => ({
      accent,
      mode,
      resolved,
      setAccent: (next: Accent) => {
        write(ACCENT_STORAGE_KEY, next)
        setAccentState(next)
      },
      setMode: (next: ThemeMode) => {
        write(MODE_STORAGE_KEY, next)
        setModeState(next)
      },
    }),
    [accent, mode, resolved],
  )

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
}
