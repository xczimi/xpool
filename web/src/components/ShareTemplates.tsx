import { useState } from 'react'
import { useI18n } from '../i18n/useI18n'
import { SHARE_TEMPLATES } from '../content/shareTemplates'

/**
 * Pool-agnostic "ready-to-send invite messages" panel on the Pools page. Each
 * curated template (see `content/shareTemplates.ts`) is shown as selectable text
 * with a Copy button, the literal `{LINK}` placeholder left for the inviter to
 * swap. Collapsed by default (`<details>`) so it doesn't crowd the pool list.
 */
export function ShareTemplates() {
  const { t } = useI18n()
  // Which template was last copied, for transient "Copied!" feedback.
  const [copiedId, setCopiedId] = useState<string | null>(null)

  const copy = (id: string, body: string) => {
    void navigator.clipboard.writeText(body)
    setCopiedId(id)
    setTimeout(() => setCopiedId((cur) => (cur === id ? null : cur)), 1500)
  }

  return (
    <details className="share-templates">
      <summary>{t('shareTemplatesTitle')}</summary>
      <p className="share-templates-hint">{t('shareTemplatesHint')}</p>
      <ul className="share-template-list">
        {SHARE_TEMPLATES.map((tpl) => (
          <li key={tpl.id} className="share-template">
            <div className="share-template-head">
              <strong>{t(tpl.labelKey)}</strong>
              <button type="button" onClick={() => copy(tpl.id, tpl.body)}>
                {copiedId === tpl.id ? t('copied') : t('copyLink')}
              </button>
            </div>
            <pre className="share-template-body">{tpl.body}</pre>
          </li>
        ))}
      </ul>
    </details>
  )
}
