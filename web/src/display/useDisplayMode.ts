import { useContext } from 'react'
import {
  DisplayModeContext,
  type DisplayModeState,
} from './displayModeContextValue'

export function useDisplayMode(): DisplayModeState {
  const ctx = useContext(DisplayModeContext)
  if (!ctx) {
    throw new Error('useDisplayMode must be used within DisplayModeProvider')
  }
  return ctx
}
