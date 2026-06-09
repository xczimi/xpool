import { useState } from 'react'
import type { FormEvent } from 'react'
import { useI18n } from '../i18n/useI18n'

/**
 * The nick + full-name form, shared by the first-run claim step and the Profile
 * page so they look and behave identically (the Profile form was the good one).
 * Presentational: it owns the field state and calls `onSubmit`; the parent owns
 * the mutation and the `flash` message. Submit is disabled while `busy` or when
 * the nick is empty.
 */
export function NameForm({
  initialNick = '',
  initialFullName = '',
  submitLabel,
  busy = false,
  flash = null,
  onSubmit,
}: {
  initialNick?: string
  initialFullName?: string
  submitLabel: string
  busy?: boolean
  flash?: string | null
  onSubmit: (nick: string, fullName: string) => void
}) {
  const { t } = useI18n()
  const [nick, setNick] = useState(initialNick)
  const [fullName, setFullName] = useState(initialFullName)
  const submit = (e: FormEvent) => {
    e.preventDefault()
    onSubmit(nick, fullName)
  }
  return (
    <>
      {flash && <p className="flash-bar">{flash}</p>}
      <form className="form" onSubmit={submit}>
        <label>
          {t('nick')}
          <input value={nick} onChange={(e) => setNick(e.target.value)} />
        </label>
        <label>
          {t('fullName')}
          <input value={fullName} onChange={(e) => setFullName(e.target.value)} />
        </label>
        <button type="submit" className="primary" disabled={busy || !nick.trim()}>
          {submitLabel}
        </button>
      </form>
    </>
  )
}
