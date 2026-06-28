import {
  composeDisplayMode,
  type ConcreteDisplayMode,
} from '../lib/displayMode'
import { useDisplayMode } from './useDisplayMode'
import { useIsMobile } from '../hooks/useIsMobile'

/** The two display axes composed into a concrete rendering for the viewport. */
export function useResolvedDisplayMode(): ConcreteDisplayMode {
  const { flag, text } = useDisplayMode()
  const isMobile = useIsMobile()
  return composeDisplayMode(flag, text, isMobile)
}
