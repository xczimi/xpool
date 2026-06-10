import { Link, useNavigate, useParams } from 'react-router-dom'
import { useMutation, useQuery } from 'urql'
import { useAuth0 } from '@auth0/auth0-react'
import { useI18n } from '../i18n/useI18n'
import { auth0Enabled } from '../auth/auth0Provider'
import { rememberPendingInvite } from '../auth/pendingInvite'
import { NameForm } from '../components/NameForm'

const ME = `query ViewerState {
  me {
    __typename
    ... on Player { id nick }
    ... on UnclaimedViewer {
      email
      linkCandidate { personId provider }
    }
  }
}`

const CLAIM = `mutation Claim($code: String!, $nick: String!, $fullName: String!) {
  claimInvite(code: $code, nick: $nick, fullName: $fullName) { player { id nick } }
}`

const JOIN = `mutation Join($code: String!) {
  join(code: $code) { id name }
}`

const LINK = `mutation Link($personId: String!) {
  confirmLink(personId: $personId) { player { id } }
}`

type PlayerViewer = { __typename: 'Player'; id: string; nick: string }
type UnclaimedViewerShape = {
  __typename: 'UnclaimedViewer'
  email?: string | null
  linkCandidate?: { personId: string; provider: string } | null
}
type ViewerShape = PlayerViewer | UnclaimedViewerShape

/**
 * The invite link is the front door to identity. This page handles every state
 * a recipient can be in:
 *  - logged out      → welcome + "Continue to join" (Auth0, preserving the code)
 *  - already a Player → accept the invite (join the pool)
 *  - unclaimed + a matching account → offer to link the login (AUTH-13)
 *  - unclaimed, new   → set a display name (shared NameForm) and claim
 */
export function InviteClaimPage() {
  const { t } = useI18n()
  const { code } = useParams<{ code: string }>()
  const navigate = useNavigate()
  const { loginWithRedirect } = useAuth0()
  const [meResult] = useQuery({ query: ME })
  const [claimResult, runClaim] = useMutation(CLAIM)
  const [joinResult, runJoin] = useMutation(JOIN)
  const [linkResult, runLink] = useMutation(LINK)

  if (!code)
    return (
      <main className="content">
        <p>{t('inviteMissingCode')}</p>
      </main>
    )
  if (meResult.fetching) return null

  const viewer = meResult.data?.me as ViewerShape | null | undefined

  // Logged out — establish identity, preserving this invite path across Auth0.
  if (!viewer) {
    const onContinue = () => {
      const returnTo = `/invite/${code}`
      // Durable breadcrumb: Auth0's `appState.returnTo` is lost if signup breaks
      // the same-tab chain (email verification opens a fresh tab). This survives.
      rememberPendingInvite(code)
      if (auth0Enabled) {
        void loginWithRedirect({
          appState: { returnTo },
          authorizationParams: { screen_hint: 'signup' },
        })
      } else {
        // Dev/e2e: no Auth0 — the auth bar's player picker is the sign-in.
        navigate('/')
      }
    }
    return (
      <main className="content">
        <h2>{t('inviteWelcomeTitle')}</h2>
        <p>{t('inviteWelcomeBody')}</p>
        <button type="button" className="primary" onClick={onContinue}>
          {t('inviteContinue')}
        </button>
      </main>
    )
  }

  // Already a Player — accept the invite (join the pool).
  if (viewer.__typename === 'Player') {
    const joinedName = joinResult.data?.join?.name as string | undefined
    return (
      <main className="content">
        <h2>{t('inviteJoinTitle')}</h2>
        {joinedName ? (
          <>
            <p>
              {t('inviteJoinedPrefix')} <strong>{joinedName}</strong>.
            </p>
            <Link to="/scoreboard">{t('inviteGoScoreboard')}</Link>
          </>
        ) : (
          <>
            <p>{t('inviteJoinBody')}</p>
            {joinResult.error && (
              <p className="flash-bar">
                {t('errorPrefix')}: {joinResult.error.message}
              </p>
            )}
            <button
              className="primary"
              disabled={joinResult.fetching}
              onClick={() => void runJoin({ code })}
            >
              {t('join')}
            </button>
          </>
        )}
      </main>
    )
  }

  // Unclaimed viewer with a matching account — offer to link (AUTH-13).
  if (viewer.linkCandidate) {
    const candidate = viewer.linkCandidate
    return (
      <main className="content">
        <h2>{t('inviteLinkTitle')}</h2>
        <p>{t('inviteLinkBody')}</p>
        {linkResult.error && (
          <p className="flash-bar">
            {t('errorPrefix')}: {linkResult.error.message}
          </p>
        )}
        <button
          className="primary"
          onClick={async () => {
            const res = await runLink({ personId: candidate.personId })
            if (!res.error) navigate('/profile')
          }}
        >
          {t('inviteLinkConfirm')}
        </button>{' '}
        <button onClick={() => navigate('/')}>{t('inviteLinkCancel')}</button>
      </main>
    )
  }

  // Unclaimed, new — set a display name and claim.
  return (
    <main className="content">
      <h2>{t('inviteClaimTitle')}</h2>
      <p>{t('inviteClaimBody')}</p>
      <NameForm
        submitLabel={t('join')}
        busy={claimResult.fetching}
        flash={
          claimResult.error
            ? `${t('errorPrefix')}: ${claimResult.error.message}`
            : null
        }
        onSubmit={async (nick, fullName) => {
          const res = await runClaim({ code, nick, fullName })
          if (!res.error) navigate('/profile')
        }}
      />
    </main>
  )
}
