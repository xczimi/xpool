import { useI18n } from '../i18n/useI18n'
import { useDisplayMode } from '../display/useDisplayMode'
import {
  FLAG_MODES,
  TEXT_MODES,
  type FlagMode,
  type TextMode,
} from '../lib/displayMode'
import type { StringKey } from '../i18n/strings'

const FLAG_LABEL: Record<FlagMode, StringKey> = {
  on: 'flagOn',
  off: 'flagOff',
}

const TEXT_LABEL: Record<TextMode, StringKey> = {
  auto: 'textAuto',
  name: 'textName',
  code: 'textCode',
  off: 'textOff',
}

/**
 * The "show" picker, split into two independent segmented toggles: whether to
 * show a flag, and what text accompanies it. Styled like the theme picker.
 * Flag off + Text off would render an empty label, so the Text `off` segment is
 * disabled while Flag is off.
 */
export function DisplayModeSelector() {
  const { t } = useI18n()
  const { flag, text, setFlag, setText } = useDisplayMode()

  return (
    <>
      <div
        className="seg-toggle"
        role="radiogroup"
        aria-label={t('displayFlag')}
      >
        {FLAG_MODES.map((f) => (
          <button
            key={f}
            type="button"
            role="radio"
            aria-checked={f === flag}
            className={`seg-option${f === flag ? ' is-active' : ''}`}
            onClick={() => setFlag(f)}
          >
            {t(FLAG_LABEL[f])}
          </button>
        ))}
      </div>
      <div
        className="seg-toggle"
        role="radiogroup"
        aria-label={t('displayText')}
      >
        {TEXT_MODES.map((m) => {
          const disabled = m === 'off' && flag === 'off'
          return (
            <button
              key={m}
              type="button"
              role="radio"
              aria-checked={m === text}
              disabled={disabled}
              className={`seg-option${m === text ? ' is-active' : ''}`}
              onClick={() => setText(m)}
            >
              {t(TEXT_LABEL[m])}
            </button>
          )
        })}
      </div>
    </>
  )
}
