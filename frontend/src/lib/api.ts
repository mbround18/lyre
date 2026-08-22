const TOKEN_KEY = 'lyre_token'

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY)
}

export function setToken(token: string) {
  localStorage.setItem(TOKEN_KEY, token)
}

export function clearToken() {
  localStorage.removeItem(TOKEN_KEY)
}

export interface ApiResponse<T> {
  success: boolean
  data?: T
  error?: string
}

class ApiError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'ApiError'
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const token = getToken()
  const res = await fetch(path, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...init?.headers,
    },
  })

  const body = (await res.json().catch(() => null)) as ApiResponse<T> | null
  if (!res.ok || !body?.success) {
    throw new ApiError(body?.error ?? `Request failed with status ${res.status}`)
  }
  return body.data as T
}

export interface Guild {
  id: string
  name: string
  icon: string | null
  owner: boolean
  permissions: string
}

export interface TrackInfo {
  title: string
  url: string
  duration: number | null
  position: number
}

export interface QueueInfo {
  guild_id: string
  current_track: TrackInfo | null
  queue: TrackInfo[]
  position: number
  is_playing: boolean
}

export interface OAuthConfig {
  client_id: string
  redirect_uri: string
}

export const api = {
  validateAuth: (accessToken: string) =>
    request<{ user: unknown; guilds: Guild[] }>('/api/auth/validate', {
      method: 'POST',
      body: JSON.stringify({ access_token: accessToken }),
    }),
  getOAuthConfig: () => request<OAuthConfig>('/api/oauth/config'),
  getGuilds: () => request<Guild[]>('/api/guilds'),
  getQueue: (guildId: string) => request<QueueInfo>(`/api/queue/${guildId}`),
  addToQueue: (guildId: string, url: string, channelId?: string) =>
    request<string>(`/api/queue/${guildId}/add`, {
      method: 'POST',
      body: JSON.stringify({ url, channel_id: channelId }),
    }),
  skipTrack: (guildId: string) =>
    request<string>(`/api/queue/${guildId}/skip`, { method: 'POST' }),
  clearQueue: (guildId: string) =>
    request<string>(`/api/queue/${guildId}`, { method: 'DELETE' }),
  stopPlayback: (guildId: string) =>
    request<string>(`/api/control/${guildId}/stop`, { method: 'POST' }),
}

export { ApiError }
