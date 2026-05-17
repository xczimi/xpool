import { useContext } from 'react'
import { I18nContext, type I18nState } from './i18nContextValue'

export function useI18n(): I18nState {
  const ctx = useContext(I18nContext)
  if (!ctx) {
    throw new Error('useI18n must be used within I18nProvider')
  }
  return ctx
}
