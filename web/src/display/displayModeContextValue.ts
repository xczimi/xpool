import { createContext } from 'react'
import type { DisplayMode } from '../lib/displayMode'

export interface DisplayModeState {
  mode: DisplayMode
  setMode: (mode: DisplayMode) => void
}

export const DisplayModeContext = createContext<DisplayModeState | undefined>(
  undefined,
)
