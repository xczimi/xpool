import { useMemo, useState, type ReactNode } from 'react'
import {
  axesFromLegacy,
  FLAG_MODES,
  TEXT_MODES,
  type DisplayAxes,
  type FlagMode,
  type TextMode,
} from '../lib/displayMode'
import {
  DisplayModeContext,
  type DisplayModeState,
} from './displayModeContextValue'

const FLAG_KEY = 'xpool.display.flag'
const TEXT_KEY = 'xpool.display.text'
const LEGACY_KEY = 'xpool.displayMode'

const DEFAULT_AXES: DisplayAxes = { flag: 'on', text: 'auto' }

const VALID_FLAG: ReadonlySet<string> = new Set(FLAG_MODES)
const VALID_TEXT: ReadonlySet<string> = new Set(TEXT_MODES)

/**
 * Resolve the initial axes from storage, migrating the legacy single-enum key
 * (`xpool.displayMode`) on first run: translate it once into the two new keys
 * and drop it. Any read/parse failure falls back to the defaults.
 */
function initialAxes(): DisplayAxes {
  try {
    const legacy = localStorage.getItem(LEGACY_KEY)
    if (legacy !== null) {
      const axes = axesFromLegacy(legacy)
      localStorage.removeItem(LEGACY_KEY)
      if (axes) {
        localStorage.setItem(FLAG_KEY, axes.flag)
        localStorage.setItem(TEXT_KEY, axes.text)
        return axes
      }
    }

    const storedFlag = localStorage.getItem(FLAG_KEY)
    const storedText = localStorage.getItem(TEXT_KEY)
    return {
      flag:
        storedFlag && VALID_FLAG.has(storedFlag)
          ? (storedFlag as FlagMode)
          : DEFAULT_AXES.flag,
      text:
        storedText && VALID_TEXT.has(storedText)
          ? (storedText as TextMode)
          : DEFAULT_AXES.text,
    }
  } catch {
    return DEFAULT_AXES
  }
}

/** Team display preference (flag + text axes) — persisted to localStorage. */
export function DisplayModeProvider({ children }: { children: ReactNode }) {
  const [{ flag, text }, setAxes] = useState(initialAxes)

  const value = useMemo<DisplayModeState>(
    () => ({
      flag,
      text,
      setFlag: (next: FlagMode) => {
        try {
          localStorage.setItem(FLAG_KEY, next)
        } catch {
          /* ignore */
        }
        setAxes((prev) => ({ ...prev, flag: next }))
      },
      setText: (next: TextMode) => {
        try {
          localStorage.setItem(TEXT_KEY, next)
        } catch {
          /* ignore */
        }
        setAxes((prev) => ({ ...prev, text: next }))
      },
    }),
    [flag, text],
  )

  return (
    <DisplayModeContext.Provider value={value}>
      {children}
    </DisplayModeContext.Provider>
  )
}
