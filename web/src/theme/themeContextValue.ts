import { createContext } from 'react'
import type { Accent, ResolvedTheme, ThemeMode } from './theme'

export interface ThemeState {
  accent: Accent
  mode: ThemeMode
  /** `mode` with `system` resolved to dark/light — what `data-theme` shows. */
  resolved: ResolvedTheme
  setAccent: (accent: Accent) => void
  setMode: (mode: ThemeMode) => void
}

export const ThemeContext = createContext<ThemeState | undefined>(undefined)
