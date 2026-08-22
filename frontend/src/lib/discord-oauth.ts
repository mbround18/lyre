import { api } from '@/lib/api'

interface OAuthMessage {
  type: 'lyre-oauth'
  token: string | null
}

function isOAuthMessage(data: unknown): data is OAuthMessage {
  return (
    typeof data === 'object' &&
    data !== null &&
    (data as { type?: unknown }).type === 'lyre-oauth'
  )
}

/**
 * Opens the Discord authorize page in a popup and resolves with the access token once
 * `/auth/callback` posts it back via `window.postMessage`. Rejects if the popup is
 * blocked, closed before completing, or Discord returns no token.
 */
export async function loginWithDiscord(): Promise<string> {
  const { client_id, redirect_uri } = await api.getOAuthConfig()

  const authorizeUrl = new URL('https://discord.com/api/oauth2/authorize')
  authorizeUrl.searchParams.set('client_id', client_id)
  authorizeUrl.searchParams.set('redirect_uri', redirect_uri)
  authorizeUrl.searchParams.set('response_type', 'code')
  authorizeUrl.searchParams.set('scope', 'identify guilds')

  const popup = window.open(authorizeUrl.toString(), 'lyre-oauth', 'width=480,height=720')
  if (!popup) {
    throw new Error('Popup was blocked - allow popups for this site and try again')
  }

  return new Promise<string>((resolve, reject) => {
    const cleanup = () => {
      window.removeEventListener('message', onMessage)
      clearInterval(closeCheck)
    }

    const onMessage = (event: MessageEvent) => {
      if (!isOAuthMessage(event.data)) return
      cleanup()
      if (event.data.token) {
        resolve(event.data.token)
      } else {
        reject(new Error('Discord did not return an access token'))
      }
    }

    const closeCheck = window.setInterval(() => {
      if (popup.closed) {
        cleanup()
        reject(new Error('Login window was closed before completing'))
      }
    }, 500)

    window.addEventListener('message', onMessage)
  })
}
