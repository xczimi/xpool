import { useEffect, useRef, useState } from 'react'
import { useI18n } from '../i18n/useI18n'
import { DisplayModeSelector } from './DisplayModeSelector'
import { LanguageSelector } from './LanguageSelector'
import { ThemeSelector } from './ThemeSelector'

/**
 * Chrome preferences (language, display flag/text, theme accent/mode) collapsed
 * behind a gear so the header stays uncluttered. Clicking the gear toggles a
 * popover of labelled rows; it closes on Escape or an outside click.
 */
export function SettingsMenu() {
  const { t } = useI18n()
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const onPointerDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false)
    }
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false)
    }
    document.addEventListener('mousedown', onPointerDown)
    document.addEventListener('keydown', onKeyDown)
    return () => {
      document.removeEventListener('mousedown', onPointerDown)
      document.removeEventListener('keydown', onKeyDown)
    }
  }, [open])

  return (
    <div className="settings-menu" ref={ref}>
      <button
        type="button"
        className="settings-gear"
        aria-label={t('settings')}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        <span aria-hidden="true">⚙</span>
        <span className="settings-gear-label">{t('settings')}</span>
      </button>
      {open && (
        <div className="settings-panel" role="dialog" aria-label={t('settings')}>
          <LanguageSelector />
          <DisplayModeSelector />
          <ThemeSelector />
        </div>
      )}
    </div>
  )
}
