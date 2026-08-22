import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { api, type Guild } from '@/lib/api'

export function GuildsPage() {
  const [guilds, setGuilds] = useState<Guild[]>([])
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    api
      .getGuilds()
      .then(setGuilds)
      .catch((err) => setError(err instanceof Error ? err.message : 'Failed to load guilds'))
  }, [])

  if (error) return <p className="text-sm text-destructive">{error}</p>

  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-2xl font-semibold">Your servers</h1>
      <div className="grid gap-3 sm:grid-cols-2">
        {guilds.map((guild) => (
          <Link key={guild.id} to={`/guild/${guild.id}`}>
            <Card className="transition-colors hover:bg-accent">
              <CardHeader>
                <CardTitle className="text-base">{guild.name}</CardTitle>
              </CardHeader>
              <CardContent className="text-sm text-muted-foreground">
                {guild.owner ? 'Owner' : 'Member'}
              </CardContent>
            </Card>
          </Link>
        ))}
        {guilds.length === 0 && (
          <p className="text-sm text-muted-foreground">No manageable servers found.</p>
        )}
      </div>
    </div>
  )
}
