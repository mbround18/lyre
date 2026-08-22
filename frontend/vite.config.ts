import path from 'node:path'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig(({ command }) => ({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  // `pnpm dev` serves at http://localhost:5173/ directly. The production build still
  // needs the /static/app/ prefix since actix-files serves it there alongside the
  // backend's other routes (see src/web_api.rs).
  base: command === 'build' ? '/static/app/' : '/',
  build: {
    outDir: '../static/app',
    emptyOutDir: true,
  },
  server: {
    proxy: {
      // Overridable so `make dev` can point this at whatever port the backend actually
      // bound to (see Makefile's DEV_HTTP_PORT) instead of a hardcoded value going stale.
      '/api': process.env.VITE_API_PROXY_TARGET ?? 'http://localhost:3100',
    },
  },
}))
