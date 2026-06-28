#!/usr/bin/env node
// `npm run e2e` entry point. Allocates two DISTINCT free TCP ports (one for the
// e2e API, one for the Vite web server), exports them as XPOOL_E2E_API_PORT /
// XPOOL_E2E_WEB_PORT, then runs Playwright as a child that inherits this env.
//
// Why a wrapper: playwright.config.ts is read before globalSetup AND re-read in
// every worker, so the port choice must be made ONCE, up front, and inherited
// by the whole Playwright process tree (config + workers + global setup/
// teardown + the spawned API & Vite). Doing it here lets multiple `npm run e2e`
// runs execute concurrently — each gets its own free ports and (via the API
// port) its own per-run state files and DynamoDB table. See e2e/ports.ts.
//
// Args after `npm run e2e --` are forwarded to `playwright test`.
import net from 'node:net'
import { spawn } from 'node:child_process'
import { createRequire } from 'node:module'
import { fileURLToPath } from 'node:url'
import { dirname, resolve } from 'node:path'

const require = createRequire(import.meta.url)
const here = dirname(fileURLToPath(import.meta.url))
const webDir = resolve(here, '..')

// Bind a server to :0 and read the OS-assigned ephemeral port.
function listenEphemeral(server) {
  return new Promise((res, rej) => {
    server.once('error', rej)
    server.listen(0, '127.0.0.1', () => res(server.address().port))
  })
}

// Allocate N distinct free ports: hold every socket open until all are assigned
// (so the OS can't hand back the same port twice), then release them. A tiny
// window remains between release and reuse; Vite's --strictPort and the API's
// bind fail loudly if it's ever lost, which doesn't happen in practice.
async function allocateDistinctPorts(n) {
  const servers = Array.from({ length: n }, () => net.createServer())
  const ports = []
  for (const s of servers) ports.push(await listenEphemeral(s))
  await Promise.all(servers.map((s) => new Promise((r) => s.close(r))))
  return ports
}

const [web, api] = await allocateDistinctPorts(2)
console.log(`[e2e] allocated dynamic ports: web=${web} api=${api}`)

const env = {
  ...process.env,
  XPOOL_E2E_WEB_PORT: String(web),
  XPOOL_E2E_API_PORT: String(api),
}

let cli
try {
  cli = require.resolve('@playwright/test/cli')
} catch {
  cli = resolve(webDir, 'node_modules/@playwright/test/cli.js')
}

const child = spawn(process.execPath, [cli, 'test', ...process.argv.slice(2)], {
  cwd: webDir,
  env,
  stdio: 'inherit',
})

child.on('exit', (code, signal) => {
  if (signal) process.kill(process.pid, signal)
  else process.exit(code ?? 1)
})
