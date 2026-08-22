import { useCallback, useEffect, useState } from 'react'
import { useParams } from 'react-router-dom'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Separator } from '@/components/ui/separator'
import { api, ApiError, type QueueInfo } from '@/lib/api'

function formatDuration(seconds: number | null) {
  if (seconds === null) return '—'
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

export function GuildPage() {
  const { guildId } = useParams<{ guildId: string }>()
  const [queue, setQueue] = useState<QueueInfo | null>(null)
  const [link, setLink] = useState('')
  const [submitting, setSubmitting] = useState(false)

  const refresh = useCallback(() => {
    if (!guildId) return
    api
      .getQueue(guildId)
      .then(setQueue)
      .catch((err) => toast.error(err instanceof Error ? err.message : 'Failed to load queue'))
  }, [guildId])

  useEffect(() => {
    refresh()
    const interval = setInterval(refresh, 5000)
    return () => clearInterval(interval)
  }, [refresh])

  const handleAdd = async (event: React.FormEvent) => {
    event.preventDefault()
    if (!guildId || !link.trim()) return
    setSubmitting(true)
    try {
      await api.addToQueue(guildId, link.trim())
      toast.success('Added to queue')
      setLink('')
      refresh()
    } catch (err) {
      toast.error(err instanceof ApiError ? err.message : 'Failed to queue link')
    } finally {
      setSubmitting(false)
    }
  }

  const handleSkip = async () => {
    if (!guildId) return
    try {
      await api.skipTrack(guildId)
      toast.success('Skipped')
      refresh()
    } catch (err) {
      toast.error(err instanceof ApiError ? err.message : 'Failed to skip')
    }
  }

  const handleStop = async () => {
    if (!guildId) return
    try {
      await api.stopPlayback(guildId)
      toast.success('Stopped')
      refresh()
    } catch (err) {
      toast.error(err instanceof ApiError ? err.message : 'Failed to stop')
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <Card>
        <CardHeader>
          <CardTitle>Queue a link</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleAdd} className="flex gap-2">
            <Input
              placeholder="Paste a YouTube / Spotify / SoundCloud link…"
              value={link}
              onChange={(e) => setLink(e.target.value)}
            />
            <Button type="submit" disabled={submitting || !link.trim()}>
              Queue
            </Button>
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Now playing</CardTitle>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={handleSkip}>
              Skip
            </Button>
            <Button variant="outline" size="sm" onClick={handleStop}>
              Stop
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {queue?.current_track ? (
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium">{queue.current_track.title}</p>
                <p className="text-sm text-muted-foreground">
                  {formatDuration(queue.current_track.duration)}
                </p>
              </div>
              <Badge variant={queue.is_playing ? 'default' : 'secondary'}>
                {queue.is_playing ? 'Playing' : 'Paused'}
              </Badge>
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">Nothing playing.</p>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Up next</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-2">
          {queue?.queue.length ? (
            queue.queue.map((track, idx) => (
              <div key={`${track.url}-${idx}`}>
                <div className="flex items-center justify-between py-2">
                  <span className="text-sm">{track.title}</span>
                  <span className="text-sm text-muted-foreground">
                    {formatDuration(track.duration)}
                  </span>
                </div>
                {idx < queue.queue.length - 1 && <Separator />}
              </div>
            ))
          ) : (
            <p className="text-sm text-muted-foreground">Queue is empty.</p>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
