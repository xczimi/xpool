import { createContext } from 'react'
import type { FlagMode, TextMode } from '../lib/displayMode'

export interface DisplayModeState {
  flag: FlagMode
  text: TextMode
  setFlag: (flag: FlagMode) => void
  setText: (text: TextMode) => void
}

export const DisplayModeContext = createContext<DisplayModeState | undefined>(
  undefined,
)
