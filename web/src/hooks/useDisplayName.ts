import { useMemo } from 'react'
import { useQuery } from 'urql'
import { PLAYERS_QUERY } from '../graphql/queries'
import type { PlayerSummary } from '../graphql/types'
import { useI18n } from '../i18n/useI18n'
import { displayNick, nickIndex } from '../lib/playerNames'

/**
 * Resolve a player id to its display nick, backed by the public `players`
 * roster. Unknown ids fall back to an "(unknown)" placeholder so a data gap is
 * visible without leaking a raw id. One shared lookup for every surface that
 * only holds player ids (pools, member lists, the transfer picker).
 */
export function useDisplayName(): (id: string) => string {
  const { t } = useI18n()
  const [{ data }] = useQuery<{ players: PlayerSummary[] }>({ query: PLAYERS_QUERY })
  const index = useMemo(() => nickIndex(data?.players ?? []), [data])
  return (id: string) => displayNick(index, id, t('unknownPlayer'))
}
