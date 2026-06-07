import { useMemo, useState, type ReactNode } from 'react'
import { DISPLAY_MODES, type DisplayMode } from '../lib/displayMode'
import {
  DisplayModeContext,
  type DisplayModeState,
} from './displayModeContextValue'

const STORAGE_KEY = 'xpool.displayMode'

const VALID: ReadonlySet<string> = new Set(DISPLAY_MODES)

function initialMode(): DisplayMode {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    if (stored && VALID.has(stored)) {
      return stored as DisplayMode
    }
  } catch {
    /* ignore */
  }
  return 'auto'
}

/** Team display-mode preference — persisted to localStorage. */
export function DisplayModeProvider({ children }: { children: ReactNode }) {
  const [mode, setModeState] = useState<DisplayMode>(initialMode())

  const value = useMemo<DisplayModeState>(
    () => ({
      mode,
      setMode: (next: DisplayMode) => {
        try {
          localStorage.setItem(STORAGE_KEY, next)
        } catch {
          /* ignore */
        }
        setModeState(next)
      },
    }),
    [mode],
  )

  return (
    <DisplayModeContext.Provider value={value}>
      {children}
    </DisplayModeContext.Provider>
  )
}
