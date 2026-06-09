import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useI18n } from '../i18n/useI18n'
import { extractCode } from './inviteCode'

/**
 * The recipient-side invite widget in isolation: paste a link or bare code,
 * `extractCode` normalises it, and we route to the public claim page
 * (`/invite/:code`). No auth coupling, no logout — usable anywhere (the Home
 * welcome, the `NeedsInvite` dead-end). See
 * docs/superpowers/specs/2026-06-09-home-identity-aware-welcome-design.md.
 */
export function InviteCodeEntry() {
  const { t } = useI18n()
  const navigate = useNavigate()
  const [entry, setEntry] = useState('')
  const [bad, setBad] = useState(false)

  const open = () => {
    const code = extractCode(entry)
    if (!code) {
      setBad(true)
      return
    }
    navigate(`/invite/${code}`)
  }

  return (
    <div className="invite-code-entry">
      <label>
        {t('inviteOnlyHaveLink')}
        <input
          type="text"
          value={entry}
          placeholder={t('inviteOnlyPastePlaceholder')}
          onChange={(e) => {
            setEntry(e.target.value)
            if (bad) setBad(false)
          }}
          onKeyDown={(e) => {
            if (e.key === 'Enter') open()
          }}
        />
      </label>
      <button type="button" onClick={open}>
        {t('inviteOnlyOpen')}
      </button>
      {bad && <p className="auth-warn">{t('inviteOnlyBadLink')}</p>}
    </div>
  )
}
