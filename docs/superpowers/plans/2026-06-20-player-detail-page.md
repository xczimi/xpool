# Player-detail page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A read-only `/player/:id` page showing one participant's whole tournament — totals + rank, per-round score strip, perfects, and an expand-on-demand per-round drill-down of their match predictions vs official results (plus group-stage standings), respecting tip-visibility gating.

**Architecture:** Pure frontend aggregation over existing GraphQL (Approach A — no backend change). The header reuses `scoreboard` + `perfects`; each collapsed round lazily fetches the existing `tips`/`standings` queries on expand, so tip-visibility gating is inherited from the `tips` resolver and never reimplemented. Pure derivation logic (rank, per-round map, filtering) lives in a unit-tested `lib/playerPage.ts`; rendering is verified by Playwright e2e (this repo has no React component-testing library — `@testing-library` is not a dependency).

**Tech Stack:** React + Vite + TypeScript, urql GraphQL client, react-router-dom, vitest (pure-logic unit tests), Playwright (e2e). i18n via `web/src/i18n/strings.ts` (en/hu).

---

## File Structure

**Create:**
- `web/src/lib/playerPage.ts` — pure derivation helpers (rank, entry lookup, per-round map, perfects filter).
- `web/src/lib/playerPage.test.ts` — vitest unit tests for the above.
- `web/src/pages/PlayerPage.tsx` — route shell: resolves viewer/own-vs-other, loads header queries, owns page-level states.
- `web/src/pages/player/PlayerHeader.tsx` — dense header (total + rank, per-round strip, perfects list).
- `web/src/pages/player/PlayerRounds.tsx` — collapsed per-round list + expansion state.
- `web/src/pages/player/PlayerRoundDetail.tsx` — one expanded round's lazy-fetched content.
- `web/src/auth/routeAccess.test.ts` — unit test for the new `/player/` access branch.
- `web/e2e/player-page.spec.ts` — end-to-end coverage.

**Modify:**
- `web/src/App.tsx` — register the `player/:id` route.
- `web/src/auth/routeAccess.ts` — treat `/player/*` as player-access.
- `web/src/i18n/strings.ts` — new en + hu keys.
- `web/src/pages/ScoreboardPage.tsx` — link each row's nick to its player page.
- `web/src/pages/AllTipsPage.tsx` — link each player column header to its player page.
- `web/src/pages/PerfectPage.tsx` — link each nick to its player page.
- `web/src/pages/ProfilePage.tsx` — add "my player page" link (own-page entry point).

No new GraphQL queries: the page composes `SCOREBOARD_QUERY`, `PERFECTS_QUERY`, `RESULTS_QUERY`, `POOLS_QUERY`, `ME_QUERY`, `TOURNAMENT_QUERY`, `TIPS_QUERY`, `STANDINGS_QUERY` — all already defined in `web/src/graphql/queries.ts`.

---

## Task 1: Pure derivation helpers

**Files:**
- Create: `web/src/lib/playerPage.ts`
- Test: `web/src/lib/playerPage.test.ts`

- [ ] **Step 1: Write the failing test**

Create `web/src/lib/playerPage.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import type { Perfect, ScoreEntry } from '../graphql/types'
import {
  perfectsOf,
  playerEntry,
  playerRank,
  rankedScoreboard,
  roundPointsOf,
} from './playerPage'

const board: ScoreEntry[] = [
  { playerId: 'a', nick: 'Ada', total: 10, stages: [{ round: 'GROUP_STAGE', points: 6 }, { round: 'R16', points: 4 }] },
  { playerId: 'b', nick: 'Bob', total: 25, stages: [{ round: 'GROUP_STAGE', points: 25 }] },
  { playerId: 'c', nick: 'Cy', total: 25, stages: [] },
]

describe('rankedScoreboard', () => {
  it('sorts by total descending without mutating the input', () => {
    const copy = [...board]
    const ranked = rankedScoreboard(board)
    expect(ranked.map((e) => e.playerId)).toEqual(['b', 'c', 'a'])
    expect(board).toEqual(copy)
  })
})

describe('playerEntry', () => {
  it('finds the entry by id', () => {
    expect(playerEntry(board, 'a')?.nick).toBe('Ada')
  })
  it('returns null when the player is absent (not a pool-mate)', () => {
    expect(playerEntry(board, 'zzz')).toBeNull()
  })
})

describe('playerRank', () => {
  it('is 1-based over the total-desc order', () => {
    expect(playerRank(board, 'b')).toBe(1)
    expect(playerRank(board, 'a')).toBe(3)
  })
  it('returns null for an absent player', () => {
    expect(playerRank(board, 'zzz')).toBeNull()
  })
})

describe('roundPointsOf', () => {
  it('maps each round to its points', () => {
    const m = roundPointsOf(board[0])
    expect(m.get('GROUP_STAGE')).toBe(6)
    expect(m.get('R16')).toBe(4)
    expect(m.get('FINAL')).toBeUndefined()
  })
})

describe('perfectsOf', () => {
  it('keeps only the given player', () => {
    const perfects: Perfect[] = [
      { playerId: 'a', nick: 'Ada', gameId: 'g1', points: 4, breakdown: { exactHome: true, exactAway: true, outcome: true, base: 4, multiplier: 1, points: 4 } },
      { playerId: 'b', nick: 'Bob', gameId: 'g1', points: 4, breakdown: { exactHome: true, exactAway: true, outcome: true, base: 4, multiplier: 1, points: 4 } },
    ]
    expect(perfectsOf(perfects, 'a').map((p) => p.playerId)).toEqual(['a'])
  })
})
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd web && npm run test -- src/lib/playerPage.test.ts`
Expected: FAIL — `Failed to resolve import "./playerPage"` (module does not exist yet).

- [ ] **Step 3: Write the implementation**

Create `web/src/lib/playerPage.ts`:

```ts
import type { Perfect, Round, ScoreEntry } from '../graphql/types'

/** Scoreboard entries ranked by total points, descending. Returns a new array. */
export function rankedScoreboard(scoreboard: ScoreEntry[]): ScoreEntry[] {
  return [...scoreboard].sort((a, b) => b.total - a.total)
}

/** A player's scoreboard entry, or null if absent (e.g. not a pool-mate). */
export function playerEntry(
  scoreboard: ScoreEntry[],
  playerId: string,
): ScoreEntry | null {
  return scoreboard.find((e) => e.playerId === playerId) ?? null
}

/** 1-based rank of a player within the total-desc scoreboard, or null. */
export function playerRank(
  scoreboard: ScoreEntry[],
  playerId: string,
): number | null {
  const idx = rankedScoreboard(scoreboard).findIndex(
    (e) => e.playerId === playerId,
  )
  return idx === -1 ? null : idx + 1
}

/** Per-round points for an entry, as a lookup map. */
export function roundPointsOf(entry: ScoreEntry): Map<Round, number> {
  return new Map(entry.stages.map((s) => [s.round, s.points]))
}

/** A single player's perfect predictions. */
export function perfectsOf(perfects: Perfect[], playerId: string): Perfect[] {
  return perfects.filter((p) => p.playerId === playerId)
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd web && npm run test -- src/lib/playerPage.test.ts`
Expected: PASS (all cases green).

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/playerPage.ts web/src/lib/playerPage.test.ts
git commit -m "feat(web): player-page derivation helpers (rank, per-round, perfects)"
```

---

## Task 2: i18n strings

**Files:**
- Modify: `web/src/i18n/strings.ts`

The new keys are added to BOTH the `en` block and the `hu` block. `StringKey` is `keyof typeof en`, so a key present in `en` but missing from `hu` is a TypeScript error — the build (`tsc -b`) is the test for this task.

- [ ] **Step 1: Add the keys to the `en` block**

In `web/src/i18n/strings.ts`, inside the `const en = { ... }` object (near the other page strings, e.g. after `perfectTitle`), add:

```ts
  // player-detail page
  playerNotInPool: 'This player is not in your pool.',
  playerNoPredictions: 'No predictions yet.',
  playerPageOwnLink: 'My player page',
  playerPerfectsHeading: 'Perfect predictions',
```

- [ ] **Step 2: Add the same keys to the `hu` block**

Inside the `hu` object, add:

```ts
  // player-detail page
  playerNotInPool: 'Ez a játékos nincs a ligádban.',
  playerNoPredictions: 'Még nincs tipp.',
  playerPageOwnLink: 'Saját oldalam',
  playerPerfectsHeading: 'Telitalálatok',
```

- [ ] **Step 3: Verify the catalogue type-checks**

Run: `cd web && npx tsc -b --noEmit`
Expected: PASS — no `Property 'playerNotInPool' is missing in type` error (which is what you'd see if a key were added to only one block).

- [ ] **Step 4: Commit**

```bash
git add web/src/i18n/strings.ts
git commit -m "feat(web): i18n strings for the player-detail page"
```

---

## Task 3: Route registration + access level

**Files:**
- Modify: `web/src/App.tsx`
- Modify: `web/src/auth/routeAccess.ts`
- Test: `web/src/auth/routeAccess.test.ts`

- [ ] **Step 1: Write the failing access-level test**

Create `web/src/auth/routeAccess.test.ts`:

```ts
import { describe, expect, it } from 'vitest'
import { accessFor } from './routeAccess'

describe('accessFor', () => {
  it('treats a player page as player-access', () => {
    expect(accessFor('/player/demo-ada')).toBe('player')
    expect(accessFor('/player/demo-ada/')).toBe('player')
  })
  it('leaves existing routes unchanged', () => {
    expect(accessFor('/scoreboard')).toBe('public')
    expect(accessFor('/admin')).toBe('admin')
    expect(accessFor('/mytips')).toBe('player')
  })
})
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd web && npm run test -- src/auth/routeAccess.test.ts`
Expected: FAIL — `expected 'public' to be 'player'` for `/player/demo-ada` (no branch yet).

- [ ] **Step 3: Add the `/player/` branch**

In `web/src/auth/routeAccess.ts`, inside `accessFor`, add the `/player/` check alongside the existing `startsWith` branches (after the `/invite` line, before the `PLAYER_PATHS` check):

```ts
  if (path === '/admin' || path.startsWith('/admin/')) return 'admin'
  if (path === '/invite' || path.startsWith('/invite/')) return 'public'
  if (path.startsWith('/player/')) return 'player'
  if (PLAYER_PATHS.has(path)) return 'player'
  return 'public'
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd web && npm run test -- src/auth/routeAccess.test.ts`
Expected: PASS.

- [ ] **Step 5: Register the route**

In `web/src/App.tsx`, add the import near the other page imports:

```ts
import { PlayerPage } from './pages/PlayerPage'
```

and add the route inside `<Route element={<Layout />}>`, immediately after the `scoreboard` route:

```tsx
        <Route path="player/:id" element={<PlayerPage />} />
```

> Note: `PlayerPage` does not exist until Task 4. This step will not type-check on its own; commit it together with Task 4, or temporarily stub `PlayerPage` with `export function PlayerPage() { return null }`. The commit below is deferred to Task 4's commit.

- [ ] **Step 6: Commit (routeAccess only)**

```bash
git add web/src/auth/routeAccess.ts web/src/auth/routeAccess.test.ts
git commit -m "feat(web): /player/* is player-access in the route map"
```

---

## Task 4: PlayerPage shell — data loading, own-vs-other, page-level states

**Files:**
- Create: `web/src/pages/PlayerPage.tsx`
- Test: covered by e2e in Task 8 (this is wiring/rendering, not pure logic).

- [ ] **Step 1: Write the implementation**

Create `web/src/pages/PlayerPage.tsx`:

```tsx
import { useMemo } from 'react'
import { useParams } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import {
  ME_QUERY,
  PERFECTS_QUERY,
  POOLS_QUERY,
  RESULTS_QUERY,
  SCOREBOARD_QUERY,
  TOURNAMENT_QUERY,
} from '../graphql/queries'
import type {
  Me,
  MatchPrediction,
  Perfect,
  Pool,
  ScoreEntry,
  Tournament,
} from '../graphql/types'
import { ErrorView, Loading, NeedsLogin } from '../components/StatusViews'
import { perfectsOf, playerEntry, playerRank } from '../lib/playerPage'
import { PlayerHeader } from './player/PlayerHeader'
import { PlayerRounds } from './player/PlayerRounds'

/**
 * One participant's complete tournament view (consumer #3). Read-only,
 * frontend-only aggregation. The header is served by `scoreboard` + `perfects`;
 * each round's predictions are lazily fetched in `PlayerRoundDetail` on expand,
 * inheriting the `tips` resolver's visibility gating. Pool-mate gating is soft:
 * a player absent from the viewer's pool scoreboard shows a "not in your pool"
 * notice instead of the page.
 */
export function PlayerPage() {
  const { id = '' } = useParams<{ id: string }>()
  const { t, locale } = useI18n()
  const { label } = useAuth()

  // Viewer's default pool (first pool they belong to); `null` = global board.
  // Mirrors ScoreboardPage's default-pool logic but without the picker.
  const [poolsResult] = useQuery<{ pools: Pool[] }>({
    query: POOLS_QUERY,
    pause: !label,
  })
  const effectivePool = poolsResult.data?.pools?.[0]?.id ?? null

  const [meResult] = useQuery<{ me: Me }>({ query: ME_QUERY, pause: !label })
  const meRaw = meResult.data?.me ?? null
  const myId = meRaw?.__typename === 'Player' ? meRaw.id : null
  const isOwn = myId !== null && myId === id

  const [scoreboardResult] = useQuery<{ scoreboard: ScoreEntry[] }>({
    query: SCOREBOARD_QUERY,
    variables: { pool: effectivePool },
    pause: !label,
  })
  const [perfectsResult] = useQuery<{ perfects: Perfect[] }>({
    query: PERFECTS_QUERY,
  })
  const [resultsResult] = useQuery<{ results: MatchPrediction[] }>({
    query: RESULTS_QUERY,
  })
  const [tournamentResult] = useQuery<{ tournament: Tournament | null }>({
    query: TOURNAMENT_QUERY,
  })

  const scoreboard = scoreboardResult.data?.scoreboard ?? []
  const entry = useMemo(() => playerEntry(scoreboard, id), [scoreboard, id])
  const rank = useMemo(() => playerRank(scoreboard, id), [scoreboard, id])
  const perfects = useMemo(
    () => perfectsOf(perfectsResult.data?.perfects ?? [], id),
    [perfectsResult.data, id],
  )
  const tournament = tournamentResult.data?.tournament ?? null
  const resultByGame = useMemo(() => {
    const map = new Map<string, MatchPrediction>()
    for (const r of resultsResult.data?.results ?? []) map.set(r.gameId, r)
    return map
  }, [resultsResult.data])

  if (!label) return <NeedsLogin />
  if (scoreboardResult.fetching || tournamentResult.fetching) return <Loading />
  if (scoreboardResult.error)
    return <ErrorView message={scoreboardResult.error.message} />
  if (!tournament) return <ErrorView />

  // Soft pool-mate gate: not the viewer and not present in their pool board.
  if (!entry) {
    return (
      <section className="page">
        <p>{t('playerNotInPool')}</p>
      </section>
    )
  }

  return (
    <section className="page player-page">
      <h2>{entry.nick}</h2>
      <PlayerHeader entry={entry} rank={rank} perfects={perfects} />
      {entry.total === 0 && perfects.length === 0 ? (
        <p>{t('playerNoPredictions')}</p>
      ) : (
        <PlayerRounds
          playerId={id}
          isOwn={isOwn}
          entry={entry}
          tournament={tournament}
          resultByGame={resultByGame}
          locale={locale}
        />
      )}
    </section>
  )
}
```

- [ ] **Step 2: Verify it type-checks**

Run: `cd web && npx tsc -b --noEmit`
Expected: FAIL — `Cannot find module './player/PlayerHeader'` and `'./player/PlayerRounds'` (created in Tasks 5–6). This is expected; do NOT commit yet. Proceed to Task 5.

---

## Task 5: PlayerHeader — total + rank, per-round strip, perfects

**Files:**
- Create: `web/src/pages/player/PlayerHeader.tsx`

- [ ] **Step 1: Write the implementation**

Create `web/src/pages/player/PlayerHeader.tsx`:

```tsx
import { useI18n } from '../../i18n/useI18n'
import type { Perfect, ScoreEntry } from '../../graphql/types'
import { roundPointsOf } from '../../lib/playerPage'
import { ROUND_ORDER, roundLabel } from '../../lib/rounds'
import { PointsBadge } from '../../components/PointsBadge'

/**
 * Dense, always-visible summary of one player: total + rank, a per-round point
 * strip (only rounds they have a score in), and their perfect predictions.
 * Pure presentation — all figures are derived upstream.
 */
export function PlayerHeader({
  entry,
  rank,
  perfects,
}: {
  entry: ScoreEntry
  rank: number | null
  perfects: Perfect[]
}) {
  const { t } = useI18n()
  const byRound = roundPointsOf(entry)
  // Show a strip cell for every round the player actually scored, in order.
  const strip = ROUND_ORDER.filter((r) => byRound.has(r))

  return (
    <div className="player-header">
      <div className="player-totals">
        <span className="player-total">
          {t('total')}: <strong>{entry.total}</strong>
        </span>
        {rank !== null && (
          <span className="player-rank">
            {t('rank')}: <strong>{rank}</strong>
          </span>
        )}
      </div>

      {strip.length > 0 && (
        <ul className="player-round-strip">
          {strip.map((r) => (
            <li key={r}>
              <span className="strip-round">{roundLabel(r, t)}</span>
              <span className="strip-points">{byRound.get(r) ?? 0}</span>
            </li>
          ))}
        </ul>
      )}

      {perfects.length > 0 && (
        <div className="player-perfects">
          <h3>
            {t('playerPerfectsHeading')} ({perfects.length})
          </h3>
          <ul className="player-perfect-list">
            {perfects.map((p) => (
              <li key={p.gameId}>
                <PointsBadge breakdown={p.breakdown} isPerfect />
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 2: Verify it type-checks**

Run: `cd web && npx tsc -b --noEmit`
Expected: still FAIL on the missing `./player/PlayerRounds` import in `PlayerPage.tsx`, but NO new errors originating in `PlayerHeader.tsx`. Proceed to Task 6.

---

## Task 6: PlayerRounds + PlayerRoundDetail — collapsed list and lazy drill-down

**Files:**
- Create: `web/src/pages/player/PlayerRounds.tsx`
- Create: `web/src/pages/player/PlayerRoundDetail.tsx`

- [ ] **Step 1: Write PlayerRoundDetail (the lazy round content)**

Create `web/src/pages/player/PlayerRoundDetail.tsx`:

```tsx
import { useMemo, useState } from 'react'
import { useQuery } from 'urql'
import { useI18n } from '../../i18n/useI18n'
import { STANDINGS_QUERY, TIPS_QUERY } from '../../graphql/queries'
import type {
  GroupGame,
  MatchPrediction,
  StandingsScore,
  Tip,
  Tournament,
} from '../../graphql/types'
import type { Locale } from '../../i18n/strings'
import { ErrorView, Loading } from '../../components/StatusViews'
import { GroupSubNav } from '../../components/GroupSubNav'
import { Matchup } from '../../components/TeamLabel'
import { PointsBadge } from '../../components/PointsBadge'
import { StandingsBadge } from '../../components/StandingsBadge'
import { byKickoff, teamIndex } from '../../lib/format'
import { leafGroupsOfRound } from '../../lib/rounds'

/**
 * One expanded round for a single player. Group Stage gets a group sub-nav and
 * loads one leaf group at a time (`tips` + `standings`); a knockout round loads
 * the round node id once (`tips` walks the subtree). Predictions are filtered
 * to `playerId`; a tip whose `prediction` is null is gated-hidden and renders
 * as a placeholder. Each fetch is lazy — this component only mounts on expand.
 */
export function PlayerRoundDetail({
  playerId,
  roundNode,
  tournament,
  resultByGame,
  locale,
}: {
  playerId: string
  roundNode: GroupGame
  tournament: Tournament
  resultByGame: Map<string, MatchPrediction>
  locale: Locale
}) {
  const { t } = useI18n()
  const isGroupStage = roundNode.round === 'GROUP_STAGE'
  const leaves = useMemo(
    () => leafGroupsOfRound(roundNode, tournament.groups),
    [roundNode, tournament.groups],
  )
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(
    () => (isGroupStage ? (leaves[0]?.id ?? null) : null),
  )

  // Group Stage queries the selected leaf group; a knockout queries the round
  // node id (its recursive `games_in` is walked server-side).
  const queryGroupId = isGroupStage ? selectedGroupId : roundNode.id

  const [tipsResult] = useQuery<{ tips: Tip[] }>({
    query: TIPS_QUERY,
    variables: { groupId: queryGroupId },
    pause: !queryGroupId,
  })
  const [standingsResult] = useQuery<{ standings: StandingsScore[] }>({
    query: STANDINGS_QUERY,
    variables: { groupId: queryGroupId },
    pause: !queryGroupId || !isGroupStage,
  })

  const teams = useMemo(
    () => teamIndex(tournament.teams, locale),
    [tournament.teams, locale],
  )
  const groupName = useMemo(() => {
    const map = new Map(tournament.groups.map((g) => [g.id, g.name]))
    return (gid: string) => map.get(gid) ?? gid
  }, [tournament.groups])

  // Only this player's tips, keyed by game.
  const tipByGame = useMemo(() => {
    const map = new Map<string, Tip>()
    for (const tip of tipsResult.data?.tips ?? []) {
      if (tip.playerId === playerId) map.set(tip.gameId, tip)
    }
    return map
  }, [tipsResult.data, playerId])
  const standings = useMemo(
    () =>
      (standingsResult.data?.standings ?? []).filter(
        (s) => s.playerId === playerId,
      ),
    [standingsResult.data, playerId],
  )

  // Which games to show: the selected group's children (group stage), or every
  // leaf game in the round (knockout), in kickoff order.
  const shownGameIds = useMemo(() => {
    const ids = isGroupStage
      ? (leaves.find((g) => g.id === selectedGroupId)?.childGameIds ?? [])
      : leaves.flatMap((g) => g.childGameIds)
    return new Set(ids)
  }, [isGroupStage, leaves, selectedGroupId])
  const games = useMemo(
    () => tournament.games.filter((g) => shownGameIds.has(g.id)).sort(byKickoff),
    [tournament.games, shownGameIds],
  )

  return (
    <div className="player-round-detail">
      {isGroupStage && (
        <GroupSubNav
          groups={leaves}
          selectedId={selectedGroupId}
          onSelect={setSelectedGroupId}
        />
      )}

      {tipsResult.fetching && <Loading />}
      {tipsResult.error && <ErrorView message={tipsResult.error.message} />}

      {!tipsResult.fetching && (
        <table className="data-table compact player-round-table">
          <thead>
            <tr>
              <th className="col-match">{t('match')}</th>
              <th>{t('player')}</th>
              <th>{t('result')}</th>
              <th>{t('points')}</th>
            </tr>
          </thead>
          <tbody>
            {games.map((g) => {
              const tip = tipByGame.get(g.id)
              const result = resultByGame.get(g.id)
              return (
                <tr key={g.id}>
                  <td>
                    <Matchup home={g.home} away={g.away} teams={teams} compact />
                  </td>
                  <td>
                    {tip?.prediction
                      ? `${tip.prediction.homeScore}–${tip.prediction.awayScore}`
                      : tip
                        ? t('hiddenTip')
                        : '—'}
                  </td>
                  <td>
                    {result ? `${result.homeScore}–${result.awayScore}` : '—'}
                  </td>
                  <td>
                    <PointsBadge
                      breakdown={tip?.breakdown}
                      isPerfect={tip?.isPerfect}
                    />
                  </td>
                </tr>
              )
            })}
          </tbody>
        </table>
      )}

      {isGroupStage && standings.length > 0 && (
        <div className="player-round-standings">
          <span>{t('standingsCol')}: </span>
          <StandingsBadge scores={standings} groupLabel={groupName} />
        </div>
      )}
    </div>
  )
}
```

- [ ] **Step 2: Write PlayerRounds (the collapsed list)**

Create `web/src/pages/player/PlayerRounds.tsx`:

```tsx
import { useMemo, useState } from 'react'
import { useI18n } from '../../i18n/useI18n'
import type {
  MatchPrediction,
  ScoreEntry,
  Tournament,
} from '../../graphql/types'
import type { Locale } from '../../i18n/strings'
import { roundPointsOf } from '../../lib/playerPage'
import { roundLabel, visibleRoundNodes } from '../../lib/rounds'
import { PlayerRoundDetail } from './PlayerRoundDetail'

/**
 * The per-round drill-down: one collapsed row per ready round showing its score,
 * expanding to a lazily-fetched `PlayerRoundDetail`. All rows start collapsed
 * (the page opens compact); each detail only mounts — and therefore only
 * fetches — when its row is opened.
 */
export function PlayerRounds({
  playerId,
  entry,
  tournament,
  resultByGame,
  locale,
}: {
  playerId: string
  isOwn: boolean
  entry: ScoreEntry
  tournament: Tournament
  resultByGame: Map<string, MatchPrediction>
  locale: Locale
}) {
  const { t } = useI18n()
  const rounds = useMemo(
    () => visibleRoundNodes(tournament.groups, tournament.games),
    [tournament.groups, tournament.games],
  )
  const byRound = roundPointsOf(entry)
  const [openId, setOpenId] = useState<string | null>(null)

  return (
    <ul className="player-rounds">
      {rounds.map((node) => {
        const isOpen = openId === node.id
        return (
          <li key={node.id} className="player-round">
            <button
              type="button"
              className="player-round-row"
              aria-expanded={isOpen}
              onClick={() => setOpenId(isOpen ? null : node.id)}
            >
              <span className="player-round-label">
                {roundLabel(node.round, t)}
              </span>
              <span className="player-round-points">
                {byRound.get(node.round) ?? 0}
              </span>
            </button>
            {isOpen && (
              <PlayerRoundDetail
                playerId={playerId}
                roundNode={node}
                tournament={tournament}
                resultByGame={resultByGame}
                locale={locale}
              />
            )}
          </li>
        )
      })}
    </ul>
  )
}
```

> `isOwn` is part of the props contract (passed by `PlayerPage`) for forward use — own pages render identically today because the `tips` resolver already returns everything for your own predictions; it is destructured-but-unused intentionally and kept off the body. If eslint flags the unused prop, prefix it in the destructure (`isOwn: _isOwn`) rather than removing it from the interface.

- [ ] **Step 3: Verify the whole web app type-checks**

Run: `cd web && npx tsc -b --noEmit`
Expected: PASS — all `PlayerPage` imports now resolve.

- [ ] **Step 4: Lint**

Run: `cd web && npm run lint`
Expected: PASS (no errors). If `isOwn` triggers `no-unused-vars`, apply the `isOwn: _isOwn` rename from the note above and re-run.

- [ ] **Step 5: Build**

Run: `cd web && npm run build`
Expected: PASS (`tsc -b && vite build` completes).

- [ ] **Step 6: Commit (the page + route together)**

```bash
git add web/src/App.tsx web/src/pages/PlayerPage.tsx web/src/pages/player/
git commit -m "feat(web): player-detail page — header + lazy per-round drill-down"
```

---

## Task 7: Entry-point links

**Files:**
- Modify: `web/src/pages/ScoreboardPage.tsx`
- Modify: `web/src/pages/AllTipsPage.tsx`
- Modify: `web/src/pages/PerfectPage.tsx`
- Modify: `web/src/pages/ProfilePage.tsx`

- [ ] **Step 1: Link scoreboard rows**

In `web/src/pages/ScoreboardPage.tsx`, add to the imports at the top:

```ts
import { Link } from 'react-router-dom'
```

Then change the nick cell in the ranked-rows map from:

```tsx
                <td>{entry.nick}</td>
```

to:

```tsx
                <td>
                  <Link to={`/player/${entry.playerId}`}>{entry.nick}</Link>
                </td>
```

- [ ] **Step 2: Link All Tips player rows**

In `web/src/pages/AllTipsPage.tsx`, add to the imports:

```ts
import { Link } from 'react-router-dom'
```

Then change the per-player row's first cell from:

```tsx
                  <td>{nick}</td>
```

to:

```tsx
                  <td>
                    <Link to={`/player/${pid}`}>{nick}</Link>
                  </td>
```

- [ ] **Step 3: Link Perfects nicks**

In `web/src/pages/PerfectPage.tsx`, add to the imports:

```ts
import { Link } from 'react-router-dom'
```

Then change the perfect row's nick cell from:

```tsx
                  <td>{p.nick}</td>
```

to:

```tsx
                  <td>
                    <Link to={`/player/${p.playerId}`}>{p.nick}</Link>
                  </td>
```

- [ ] **Step 4: Add the own-page link to Profile**

In `web/src/pages/ProfilePage.tsx`, add to the imports:

```ts
import { Link } from 'react-router-dom'
```

Then, inside the returned `<section className="page">` of the `ProfilePage` component, after `<ProfileForm key={me.id} me={me} />`, add:

```tsx
      <p>
        <Link to={`/player/${me.id}`}>{t('playerPageOwnLink')}</Link>
      </p>
```

- [ ] **Step 5: Verify build + lint**

Run: `cd web && npx tsc -b --noEmit && npm run lint`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add web/src/pages/ScoreboardPage.tsx web/src/pages/AllTipsPage.tsx web/src/pages/PerfectPage.tsx web/src/pages/ProfilePage.tsx
git commit -m "feat(web): link nicks to player-detail page from scoreboard, all-tips, perfects, profile"
```

---

## Task 8: End-to-end coverage

**Files:**
- Create: `web/e2e/player-page.spec.ts`

This follows the established e2e pattern (`web/e2e/points-on-tips.spec.ts`): seed the `balanced` scenario into the table the live stack booted, drive the real stack, and assert on rendered content while `watchNetwork` guards against GraphQL/wire faults. The `balanced` scenario seeds the demo roster (`demo-ada`, `demo-alan`, …) with full predictions and official results.

The two clock pins exercise both sides of the visibility gate via the server-authoritative clock:
- `AFTER_TOURNAMENT` — every match has opened, so another player's picks are revealed.
- `BEFORE_TOURNAMENT` — no match has opened and the viewer/other haven't mutually locked, so another player's group-stage picks render as the `hidden` placeholder.

- [ ] **Step 1: Write the spec**

Create `web/e2e/player-page.spec.ts`:

```ts
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test, expect } from '@playwright/test'
import { devLogin, expectNoErrorView, watchNetwork } from './helpers'

/**
 * Player-detail page (#3) end to end. Seeds the `balanced` scenario (full demo
 * roster + official results) and drives the real stack.
 */
const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = resolve(here, '../..')

/** A clock past the Final: every match played → another player's picks visible. */
const AFTER_TOURNAMENT = '2026-07-20T00:00:00Z'
/** A clock before kickoff: no match opened → another's picks stay gated-hidden. */
const BEFORE_TOURNAMENT = '2026-06-01T00:00:00Z'

test.beforeAll(() => {
  const table = readFileSync(resolve(repoRoot, 'web/.e2e-table'), 'utf8').trim()
  execFileSync('cargo', ['run', '-p', 'xtask', '--', 'scenario', 'balanced'], {
    cwd: repoRoot,
    stdio: 'inherit',
    env: {
      ...process.env,
      XPOOL_TABLE: table,
      DYNAMO_ENDPOINT: 'http://localhost:8001',
    },
  })
})

test('own page shows totals, a per-round strip, and drill-down with points', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.addInitScript(
    (v) => localStorage.setItem('xpool.devNow', v),
    AFTER_TOURNAMENT,
  )
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/player/demo-ada')

  // Header: total + per-round strip rendered.
  await expect(page.locator('.player-header')).toBeVisible()
  await expect(page.locator('.player-round-strip li').first()).toBeVisible()

  // Rounds start collapsed; expanding the Group Stage row reveals its detail
  // (lazy fetch), with a group sub-nav and scored predictions.
  const firstRow = page.locator('.player-round-row').first()
  await expect(firstRow).toHaveAttribute('aria-expanded', 'false')
  await firstRow.click()
  await expect(page.locator('.player-round-detail')).toBeVisible()
  await expect(page.locator('.group-subnav')).toBeVisible()
  await expect(page.locator('.player-round-detail .points-badge').first()).toBeVisible()

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('reaches a pool-mate page from a scoreboard link; picks visible after kickoff', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.addInitScript(
    (v) => localStorage.setItem('xpool.devNow', v),
    AFTER_TOURNAMENT,
  )
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/scoreboard')

  // Clicking another player's name navigates to their page.
  await page.getByRole('link', { name: 'demo-alan' }).first().click()
  await expect(page).toHaveURL(/\/player\/demo-alan$/)
  await expect(page.locator('.player-header')).toBeVisible()

  // After the tournament every match opened → their group-stage picks are
  // revealed (a real scoreline, not the hidden placeholder).
  await page.locator('.player-round-row').first().click()
  await expect(page.locator('.player-round-detail')).toBeVisible()
  await expect(page.locator('.player-round-table tbody tr').first()).not.toContainText(
    'hidden',
  )

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})

test('another player’s un-revealable picks are hidden before kickoff', async ({
  page,
}) => {
  const net = watchNetwork(page)
  await page.addInitScript(
    (v) => localStorage.setItem('xpool.devNow', v),
    BEFORE_TOURNAMENT,
  )
  await page.goto('/')
  await devLogin(page, 'demo-ada')
  await page.goto('/player/demo-alan')

  // Before any kickoff, the viewer and demo-alan have not mutually locked, so
  // the tips resolver gates demo-alan's picks → placeholder cells.
  await page.locator('.player-round-row').first().click()
  await expect(page.locator('.player-round-detail')).toBeVisible()
  await expect(page.locator('.player-round-table')).toContainText('hidden')

  await expectNoErrorView(page)
  await net.assertNoGraphqlErrors()
  net.assertNoPageErrors()
})
```

- [ ] **Step 2: Run the e2e suite (this spec)**

Run: `cd web && npm run e2e -- player-page`
Expected: PASS — all three tests green. (`npm run e2e` boots the full live stack via `e2e/global-setup.ts` on its isolated ports; the `beforeAll` reseeds the booted table with the `balanced` scenario.)

> If the "before kickoff" test sees scorelines instead of `hidden`, confirm the `balanced` scenario leaves `demo-alan`'s group-stage predictions effective-locked-independent of `demo-ada` — i.e. the gate depends on mutual lock. If the scenario auto-locks all players, adjust the assertion to view a player who is NOT a pool-mate of `demo-ada` (the soft pool gate then shows `playerNotInPool`), and keep the reveal assertion in the AFTER test. Pick whichever the seeded data supports; do not weaken the visible-after-kickoff assertion.

- [ ] **Step 3: Commit**

```bash
git add web/e2e/player-page.spec.ts
git commit -m "test(e2e): player-detail page — header, drill-down, entry link, visibility gate"
```

---

## Task 9: Final verification

- [ ] **Step 1: Full web checks**

Run: `cd web && npx tsc -b --noEmit && npm run lint && npm run build && npm run test`
Expected: all PASS.

- [ ] **Step 2: Confirm the feature manually (optional but recommended)**

Run the stack (`bin/local-dev`), log in as `demo-ada`, visit `/scoreboard`, click a name, expand a round. Confirm: collapsed-by-default rows, lazy expansion, points badges, group sub-nav on Group Stage, and the "my player page" link on `/profile`.

- [ ] **Step 3: Flip the PRD status**

Edit `.scratch/player-detail-page/PRD.md`: change `Status: needs-triage` to `Status: done` (or `ready-for-human` review). Commit:

```bash
git add .scratch/player-detail-page/PRD.md
git commit -m "docs(issue): player-detail page (#3) implemented"
```

---

## Self-Review Notes

- **Spec coverage:** route `/player/:id` (T3); summary-first header with total+rank, per-round strip, perfects count+list (T5); all-collapsed drill-down (T6/PlayerRounds); knockout via round-node id, group-stage via GroupSubNav + tips+standings interleaved (T6/PlayerRoundDetail); Approach A / no backend (all tasks); pool-mate soft gate + hard pick gate inherited from `tips` (T4 + T6 placeholder rendering); entry points scoreboard/all-tips/perfects/profile (T7); empty + error states (T4); i18n en+hu (T2); e2e + unit tests (T1, T3, T8). All spec sections map to a task.
- **No placeholders:** every code step shows complete code; commands have expected output.
- **Type consistency:** `playerEntry`/`playerRank`/`roundPointsOf`/`perfectsOf`/`rankedScoreboard` defined in T1 and consumed unchanged in T4/T5; `PlayerRoundDetail` and `PlayerRounds` prop shapes match the call sites in `PlayerPage`/`PlayerRounds`; query names (`SCOREBOARD_QUERY`, `PERFECTS_QUERY`, `RESULTS_QUERY`, `POOLS_QUERY`, `ME_QUERY`, `TOURNAMENT_QUERY`, `TIPS_QUERY`, `STANDINGS_QUERY`) and type names (`ScoreEntry`, `Perfect`, `Tip`, `StandingsScore`, `MatchPrediction`, `GroupGame`, `Tournament`, `Locale`) all match `web/src/graphql/{queries,types}.ts`.
