import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Separator } from '@/components/ui/separator'
import { api, ApiError } from '@/lib/api'
import { useAuth } from '@/lib/auth'
import { loginWithDiscord } from '@/lib/discord-oauth'

export function LoginPage() {
  const [accessToken, setAccessToken] = useState('')
  const [loading, setLoading] = useState(false)
  const [discordLoading, setDiscordLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const { setToken } = useAuth()
  const navigate = useNavigate()

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault()
    setLoading(true)
    setError(null)
    try {
      await api.validateAuth(accessToken)
      setToken(accessToken)
      navigate('/')
    } catch (err) {
      setError(err instanceof ApiError ? err.message : 'Failed to authenticate')
    } finally {
      setLoading(false)
    }
  }

  const handleDiscordLogin = async () => {
    setDiscordLoading(true)
    setError(null)
    try {
      const token = await loginWithDiscord()
      setToken(token)
      navigate('/')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Discord sign-in failed')
    } finally {
      setDiscordLoading(false)
    }
  }

  return (
    <div className="mx-auto max-w-sm">
      <Card>
        <CardHeader>
          <CardTitle>Sign in</CardTitle>
          <CardDescription>Sign in with Discord to manage your servers.</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <Button onClick={handleDiscordLogin} disabled={discordLoading}>
            {discordLoading ? 'Waiting for Discord…' : 'Continue with Discord'}
          </Button>

          <div className="flex items-center gap-3">
            <Separator className="flex-1" />
            <span className="text-xs text-muted-foreground">or</span>
            <Separator className="flex-1" />
          </div>

          <form onSubmit={handleSubmit} className="flex flex-col gap-4">
            <Input
              placeholder="Paste an access token (or a demo_ token in dev)"
              value={accessToken}
              onChange={(e) => setAccessToken(e.target.value)}
            />
            <Button type="submit" variant="outline" disabled={loading || !accessToken}>
              {loading ? 'Signing in…' : 'Sign in with token'}
            </Button>
          </form>

          {error && <p className="text-sm text-destructive">{error}</p>}
        </CardContent>
      </Card>
    </div>
  )
}
