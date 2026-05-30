import { useQuery } from 'urql'

const ME_STATUS = `query ViewerStatus {
  me {
    __typename
    ... on UnclaimedViewer {
      linkCandidate { personId }
    }
  }
}`

type ViewerStatus =
  | { __typename: 'Player' }
  | { __typename: 'UnclaimedViewer'; linkCandidate?: { personId: string } | null }

export function UnclaimedBanner() {
  const [result] = useQuery({ query: ME_STATUS })
  const me = result.data?.me as ViewerStatus | null | undefined
  if (!me || me.__typename !== 'UnclaimedViewer') return null
  if (me.linkCandidate) return null // AUTH-13 flow is handled on the invite page
  return (
    <div className="banner">
      You're signed in, but you need an invitation to play. Ask a friend who
      plays for an invite link.
    </div>
  )
}
