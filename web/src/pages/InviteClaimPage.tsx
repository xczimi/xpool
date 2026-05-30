import { useParams } from 'react-router-dom'
import { useMutation, useQuery } from 'urql'
import { useState } from 'react'

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

export function InviteClaimPage() {
  const { code } = useParams<{ code: string }>()
  const [meResult] = useQuery({ query: ME })
  const [claimResult, runClaim] = useMutation(CLAIM)
  const [linkResult, runLink] = useMutation(LINK)
  const [nick, setNick] = useState('')
  const [fullName, setFullName] = useState('')

  if (!code) return <p>missing invite code</p>
  if (meResult.fetching) return null

  const viewer = meResult.data?.me as ViewerShape | null | undefined

  if (!viewer) {
    return (
      <main className="content">
        <h2>Claim your invite</h2>
        <p>Log in to claim this invite.</p>
        <a href="/">Go to log in</a>
      </main>
    )
  }

  if (viewer.__typename === 'Player') {
    return (
      <main className="content">
        <h2>Welcome back</h2>
        <p>You're already in xPool.</p>
      </main>
    )
  }

  // Unclaimed viewer with a link candidate — AUTH-13 flow
  if (viewer.linkCandidate) {
    const candidate = viewer.linkCandidate
    return (
      <main className="content">
        <h2>Link this login?</h2>
        <p>
          An account already exists for {viewer.email}, signed in via{' '}
          {candidate.provider}. Link this login to that account?
        </p>
        {linkResult.error && (
          <p className="flash-bar">Error: {linkResult.error.message}</p>
        )}
        <button
          onClick={async () => {
            const res = await runLink({ personId: candidate.personId })
            if (!res.error) window.location.href = '/profile'
          }}
        >
          Yes, link
        </button>
        <button onClick={() => (window.location.href = '/')}>
          No, cancel
        </button>
      </main>
    )
  }

  // Unclaimed viewer without a link candidate — standard claim flow
  return (
    <main className="content">
      <h2>Claim your invite</h2>
      <p>Set your display name.</p>
      {claimResult.error && (
        <p className="flash-bar">Error: {claimResult.error.message}</p>
      )}
      <input
        value={nick}
        onChange={(e) => setNick(e.target.value)}
        placeholder="Nick"
      />
      <input
        value={fullName}
        onChange={(e) => setFullName(e.target.value)}
        placeholder="Full name"
      />
      <button
        disabled={claimResult.fetching || !nick.trim()}
        onClick={async () => {
          const res = await runClaim({ code, nick, fullName })
          if (!res.error) window.location.href = '/profile'
        }}
      >
        Claim
      </button>
    </main>
  )
}
