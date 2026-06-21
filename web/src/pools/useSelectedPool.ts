import { useContext } from 'react'
import {
  SelectedPoolContext,
  type SelectedPoolState,
} from './selectedPoolContextValue'

export function useSelectedPool(): SelectedPoolState {
  const ctx = useContext(SelectedPoolContext)
  if (!ctx) {
    throw new Error('useSelectedPool must be used within SelectedPoolProvider')
  }
  return ctx
}
