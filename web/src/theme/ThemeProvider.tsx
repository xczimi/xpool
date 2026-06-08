import { useEffect, useMemo, useState, useSyncExternalStore, type ReactNode } from 'react'
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
  const prefersDark = useSyncExternalStore(
    (onStoreChange) => {
      // Only react to OS changes while following the system preference.
      if (mode !== 'system') return () => {}
      const mql = window.matchMedia(DARK_QUERY)
      mql.addEventListener('change', onStoreChange)
      return () => mql.removeEventListener('change', onStoreChange)
    },
    getSystemPrefersDark,
    () => false,
  )

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
