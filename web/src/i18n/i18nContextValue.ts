import { createContext } from 'react'
import type { Locale, StringKey } from './strings'

export interface I18nState {
  locale: Locale
  setLocale: (locale: Locale) => void
  t: (key: StringKey) => string
}

export const I18nContext = createContext<I18nState | undefined>(undefined)
