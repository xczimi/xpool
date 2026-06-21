import { useMemo, useState, type ReactNode } from 'react'
import { readSelectedPool, writeSelectedPool } from '../lib/selectedPool'
import {
  SelectedPoolContext,
  type SelectedPoolState,
} from './selectedPoolContextValue'

/** Sticky, cross-page pool selection — persisted to localStorage. */
export function SelectedPoolProvider({ children }: { children: ReactNode }) {
  const [selected, setSelectedState] = useState<string | null | undefined>(
    readSelectedPool,
  )

  const value = useMemo<SelectedPoolState>(
    () => ({
      selected,
      setSelected: (poolId: string | null) => {
        writeSelectedPool(poolId)
        setSelectedState(poolId)
      },
    }),
    [selected],
  )

  return (
    <SelectedPoolContext.Provider value={value}>
      {children}
    </SelectedPoolContext.Provider>
  )
}
