import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import { Toaster } from '@/components/ui/sonner'
import { Layout } from '@/components/layout'
import { RequireAuth } from '@/components/require-auth'
import { AuthProvider } from '@/lib/auth'
import { LoginPage } from '@/pages/login'
import { GuildsPage } from '@/pages/guilds'
import { GuildPage } from '@/pages/guild'

function App() {
  return (
    <AuthProvider>
      <BrowserRouter basename={import.meta.env.BASE_URL}>
        <Toaster />
        <Routes>
          <Route element={<Layout />}>
            <Route path="/login" element={<LoginPage />} />
            <Route element={<RequireAuth />}>
              <Route index element={<GuildsPage />} />
              <Route path="/guild/:guildId" element={<GuildPage />} />
            </Route>
            <Route path="*" element={<Navigate to="/" replace />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </AuthProvider>
  )
}

export default App
