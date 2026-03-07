import { fork, execSync, type ChildProcess } from 'node:child_process'
import { existsSync } from 'node:fs'
import { networkInterfaces } from 'node:os'
import { join, resolve } from 'node:path'

let mcpProcess: ChildProcess | null = null
let mcpPort: number | null = null

/** Resolve the MCP server script path across dev, web build, and Electron production. */
function resolveMcpServerScript(): string {
  // Electron production: extraResources
  const electronResources = process.env.ELECTRON_RESOURCES_PATH
  if (electronResources) {
    const p = join(electronResources, 'mcp-server.cjs')
    if (existsSync(p)) return p
  }
  // dev + web build
  const fromCwd = resolve(process.cwd(), 'dist', 'mcp-server.cjs')
  if (existsSync(fromCwd)) return fromCwd
  // Fallback: relative to this file (Nitro bundled output)
  const fromFile = resolve(__dirname, '..', '..', '..', 'dist', 'mcp-server.cjs')
  if (existsSync(fromFile)) return fromFile
  return fromCwd
}

/** Get the first non-internal IPv4 address (LAN IP). */
export function getLocalIp(): string | null {
  const nets = networkInterfaces()
  for (const name of Object.keys(nets)) {
    for (const net of nets[name] ?? []) {
      if (net.family === 'IPv4' && !net.internal) {
        return net.address
      }
    }
  }
  return null
}

export function getMcpServerStatus(): { running: boolean; port: number | null; localIp: string | null } {
  const running = mcpProcess !== null && mcpProcess.exitCode === null
  return {
    running,
    port: running ? mcpPort : null,
    localIp: running ? getLocalIp() : null,
  }
}

export function startMcpHttpServer(port: number): { running: boolean; port: number; localIp: string | null; error?: string } {
  if (mcpProcess && mcpProcess.exitCode === null) {
    return { running: true, port: mcpPort!, localIp: getLocalIp() }
  }

  const serverScript = resolveMcpServerScript()

  try {
    mcpProcess = fork(serverScript, ['--http', '--port', String(port)], {
      stdio: ['ignore', 'pipe', 'pipe', 'ipc'],
      env: { ...process.env },
    })

    mcpPort = port

    mcpProcess.stderr?.on('data', (data: Buffer) => {
      console.error(`[mcp-server] ${data.toString().trim()}`)
    })

    mcpProcess.on('exit', (code) => {
      console.error(`[mcp-server] exited with code ${code}`)
      mcpProcess = null
      mcpPort = null
    })

    return { running: true, port, localIp: getLocalIp() }
  } catch (err) {
    return { running: false, port, localIp: null, error: err instanceof Error ? err.message : String(err) }
  }
}

export function stopMcpHttpServer(): { running: false } {
  if (mcpProcess) {
    if (process.platform === 'win32') {
      // SIGTERM is unreliable on Windows; use taskkill for proper cleanup
      try {
        const pid = mcpProcess.pid
        if (pid) {
          execSync(`taskkill /pid ${pid} /T /F`, { stdio: 'ignore' })
        }
      } catch { /* ignore */ }
    } else {
      mcpProcess.kill('SIGTERM')
    }
    mcpProcess = null
    mcpPort = null
  }
  return { running: false }
}
