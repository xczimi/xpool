import { useContext } from 'react'
import { ThemeContext, type ThemeState } from './themeContextValue'

export function useTheme(): ThemeState {
  const ctx = useContext(ThemeContext)
  if (!ctx) {
    throw new Error('useTheme must be used within ThemeProvider')
  }
  return ctx
}
