import { useMutation } from 'urql'

const CREATE_INVITE = `mutation CreateInvite { createInvite(pool: null) { code link } }`

export function InvitePage() {
  const [result, run] = useMutation(CREATE_INVITE)
  const link = result.data?.createInvite?.link as string | undefined
  return (
    <main className="content">
      <h2>Invite</h2>
      <button onClick={() => void run({})}>Generate link</button>
      {link && (
        <>
          <p>Share this link with your friend:</p>
          <textarea readOnly value={link} onFocus={(e) => e.currentTarget.select()} />
          <button onClick={() => void navigator.clipboard.writeText(link)}>Copy</button>
        </>
      )}
    </main>
  )
}
