import { useI18n } from '../i18n/useI18n'
import { useTheme } from '../theme/useTheme'
import { ACCENTS, THEME_MODES, type Accent, type ThemeMode } from '../theme/theme'
import type { StringKey } from '../i18n/strings'
import { SegRow, SegToggle } from './SegToggle'

const ACCENT_LABEL: Record<Accent, StringKey> = {
  amber: 'accentAmber',
  green: 'accentGreen',
  cyan: 'accentCyan',
  magenta: 'accentMagenta',
  violet: 'accentViolet',
  mono: 'accentMono',
}

const MODE_LABEL: Record<ThemeMode, StringKey> = {
  system: 'modeSystem',
  dark: 'modeDark',
  light: 'modeLight',
}

export function ThemeSelector() {
  const { t } = useI18n()
  const { accent, mode, setAccent, setMode } = useTheme()
  return (
    <>
      <SegRow label={t('theme')}>
        <div
          className="accent-swatches"
          role="radiogroup"
          aria-label={t('theme')}
        >
          {ACCENTS.map((a) => (
            <button
              key={a}
              type="button"
              role="radio"
              aria-checked={a === accent}
              aria-label={t(ACCENT_LABEL[a])}
              title={t(ACCENT_LABEL[a])}
              className={`accent-swatch accent-swatch-${a}${a === accent ? ' is-active' : ''}`}
              onClick={() => setAccent(a)}
            />
          ))}
        </div>
      </SegRow>
      <SegToggle
        label={t('mode')}
        options={THEME_MODES}
        value={mode}
        onChange={setMode}
        renderOption={(m) => t(MODE_LABEL[m])}
      />
    </>
  )
}
