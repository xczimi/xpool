import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// The API port is a dev/e2e knob: dev runs the API on :3000, the Playwright
// e2e stack on :3001 so the two stacks coexist. `XPOOL_API_PORT` retargets the
// proxy; the dev web port stays Vite's default (:5173) while e2e passes
// `--port 5174`. See web/scripts/e2e-stack.sh and playwright.config.ts.
const apiPort = process.env.XPOOL_API_PORT ?? '3000'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      '/api': {
        target: `http://localhost:${apiPort}`,
        changeOrigin: true,
      },
    },
  },
})
