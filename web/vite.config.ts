import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// The API port is a dev/e2e knob: dev runs the API on :3000; the Playwright
// e2e stack runs on a per-run DYNAMIC port (legacy fallback :3001) so stacks —
// and concurrent e2e runs — coexist. `XPOOL_API_PORT` retargets the proxy;
// playwright.config.ts passes the run's API port, and the Vite web port is
// likewise dynamic (`--port <web>`, legacy fallback :5173/:5174). See
// web/e2e/ports.ts, web/scripts/e2e-stack.sh and playwright.config.ts.
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
