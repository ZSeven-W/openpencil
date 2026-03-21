/**
 * Electron development workflow orchestrator.
 *
 * 1. Start Vite dev server (bun run dev)
 * 2. Wait for it to be ready on port 3000
 * 3. Compile electron/ with esbuild
 * 4. Launch Electron pointing at the dev server
 */

import { spawn, execSync, type ChildProcess } from 'node:child_process'
import { build } from 'esbuild'
import { join } from 'node:path'

const DESKTOP_DIR = import.meta.dirname
const ROOT = join(DESKTOP_DIR, '..', '..')
const VITE_DEV_PORT = 3000

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async function waitForServer(
  url: string,
  timeoutMs = 30_000,
): Promise<void> {
  const start = Date.now()
  while (Date.now() - start < timeoutMs) {
    try {
      const res = await fetch(url)
      if (res.ok || res.status < 500) return
    } catch {
      // server not ready yet
    }
    await new Promise((r) => setTimeout(r, 500))
  }
  throw new Error(`Timeout waiting for ${url}`)
}

async function compileElectron(): Promise<void> {
  const common: Parameters<typeof build>[0] = {
    platform: 'node',
    bundle: true,
    sourcemap: true,
    external: ['electron'],
    target: 'node20',
    outdir: join(ROOT, 'out', 'desktop'),
    outExtension: { '.js': '.cjs' },
    format: 'cjs' as const,
  }

  await Promise.all([
    build({
      ...common,
      entryPoints: [join(DESKTOP_DIR, 'main.ts')],
    }),
    build({
      ...common,
      entryPoints: [join(DESKTOP_DIR, 'preload.ts')],
    }),
  ])

  console.log('[electron-dev] Electron files compiled')
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
  // 1. Start Vite dev server
  console.log('[electron-dev] Starting Vite dev server...')
  const vite = spawn('bun', ['--bun', 'run', 'dev'], {
    cwd: ROOT,
    stdio: 'inherit',
    env: { ...process.env },
  })

  // Ensure cleanup on exit
  const cleanup = () => {
    if (process.platform === 'win32' && vite.pid) {
      try {
        execSync(`taskkill /pid ${vite.pid} /T /F`, { stdio: 'ignore' })
      } catch { /* ignore */ }
    } else {
      vite.kill()
    }
    process.exit()
  }
  process.on('SIGINT', cleanup)
  process.on('SIGTERM', cleanup)

  // 2. Wait for Vite to be ready
  console.log(`[electron-dev] Waiting for Vite on port ${VITE_DEV_PORT}...`)
  await waitForServer(`http://localhost:${VITE_DEV_PORT}`)
  console.log('[electron-dev] Vite is ready')

  // 3. Compile MCP server + Electron files
  console.log('[electron-dev] Compiling MCP server...')
  await build({
    platform: 'node',
    bundle: true,
    sourcemap: true,
    target: 'node20',
    format: 'cjs',
    entryPoints: [join(ROOT, 'apps', 'web', 'src', 'mcp', 'server.ts')],
    outfile: join(ROOT, 'out', 'mcp-server.cjs'),
    alias: { '@': join(ROOT, 'apps', 'web', 'src') },
    define: { 'import.meta.env': '{}' },
    external: ['canvas', 'paper'],
  })
  console.log('[electron-dev] MCP server compiled')

  await compileElectron()

  // 4. Launch Electron
  console.log('[electron-dev] Starting Electron...')
  const electronBin = join(ROOT, 'node_modules', '.bin', 'electron')
  const electron = spawn(electronBin, [join(ROOT, 'out', 'desktop', 'main.cjs')], {
    cwd: ROOT,
    stdio: 'inherit',
    env: { ...process.env },
  }) as ChildProcess

  electron.on('exit', () => {
    if (process.platform === 'win32' && vite.pid) {
      try {
        execSync(`taskkill /pid ${vite.pid} /T /F`, { stdio: 'ignore' })
      } catch { /* ignore */ }
    } else {
      vite.kill()
    }
    process.exit()
  })
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
