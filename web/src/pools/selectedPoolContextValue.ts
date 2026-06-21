import { createContext } from 'react'

export interface SelectedPoolState {
  /** The raw three-state selection: pool id, `null` (everyone), `undefined` (unchosen). */
  selected: string | null | undefined
  /** Set the explicit selection (a pool id, or `null` for everyone). Persists. */
  setSelected: (poolId: string | null) => void
}

export const SelectedPoolContext = createContext<SelectedPoolState | undefined>(
  undefined,
)
