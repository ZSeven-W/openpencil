import { spawn } from 'node:child_process'
import { existsSync, writeFileSync, unlinkSync, readFileSync } from 'node:fs'
import { networkInterfaces } from 'node:os'
import { join, resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
<<<<<<< fix/mcp-server-detached-process
import { tmpdir } from 'node:os'
=======

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)
>>>>>>> v0.4.0

// ESM-compatible __dirname polyfill
const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

// PID/Port files for tracking the detached MCP server process across restarts
const MCP_PID_FILE = join(tmpdir(), 'openpencil-mcp-server.pid')
const MCP_PORT_FILE = join(tmpdir(), 'openpencil-mcp-server.port')

/** Resolve the MCP server script path across dev, web build, and Electron production. */
function resolveMcpServerScript(): string {
  // Electron production: extraResources
  const electronResources = process.env.ELECTRON_RESOURCES_PATH
  if (electronResources) {
    const p = join(electronResources, 'mcp-server.cjs')
    if (existsSync(p)) return p
  }
  // dev + web build (from cwd)
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

/** Check if a process with the given PID is running. */
function isProcessRunning(pid: number): boolean {
  try {
    // Signal 0 checks existence without actually sending a signal
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}

/** Read PID from file if it exists and process is still running. */
function getRunningPid(): { pid: number; port: number } | null {
  try {
    if (!existsSync(MCP_PID_FILE)) return null
    const pid = parseInt(readFileSync(MCP_PID_FILE, 'utf-8').trim(), 10)
    if (isNaN(pid) || !isProcessRunning(pid)) {
      // Stale PID file - clean up
      try { unlinkSync(MCP_PID_FILE) } catch { /* ignore */ }
      try { unlinkSync(MCP_PORT_FILE) } catch { /* ignore */ }
      return null
    }
    const port = existsSync(MCP_PORT_FILE)
      ? parseInt(readFileSync(MCP_PORT_FILE, 'utf-8').trim(), 10)
      : 3100
    return { pid, port: isNaN(port) ? 3100 : port }
  } catch {
    return null
  }
}

export function getMcpServerStatus(): { running: boolean; port: number | null; localIp: string | null } {
  const info = getRunningPid()
  if (!info) {
    return { running: false, port: null, localIp: null }
  }
  return { running: true, port: info.port, localIp: getLocalIp() }
}

export function startMcpHttpServer(port: number): { running: boolean; port: number; localIp: string | null; error?: string } {
  // Check if already running
  const existing = getRunningPid()
  if (existing) {
    return { running: true, port: existing.port, localIp: getLocalIp() }
  }

  const serverScript = resolveMcpServerScript()

  try {
    // CRITICAL: Use detached mode with unref() so the MCP server survives
    // independently of the parent Nitro process. This prevents the server
    // from dying when:
    // 1. The UI settings dialog is closed
    // 2. The user interacts with the editor canvas
    // 3. The Nitro server restarts or hot-reloads
    // 4. The Electron app sends SIGTERM to Nitro on window close
    //
    // The process runs in its own session and writes its PID to a file
    // for later tracking and graceful shutdown.
    const child = spawn(process.execPath, [serverScript, '--http', '--port', String(port)], {
      stdio: ['ignore', 'ignore', 'ignore'],
      env: { ...process.env },
      detached: true,
      windowsHide: true,
    })

    // Allow parent to exit independently of child
    child.unref()

    // Write PID to file for later tracking (after brief delay to ensure startup)
    const childPid = child.pid
    if (childPid) {
      setTimeout(() => {
        try {
          if (isProcessRunning(childPid)) {
            writeFileSync(MCP_PID_FILE, String(childPid), 'utf-8')
            writeFileSync(MCP_PORT_FILE, String(port), 'utf-8')
          }
        } catch { /* ignore write errors */ }
      }, 100)
    }

    return { running: true, port, localIp: getLocalIp() }
  } catch (err) {
    return { running: false, port, localIp: null, error: err instanceof Error ? err.message : String(err) }
  }
}

export function stopMcpHttpServer(): { running: false } {
  const info = getRunningPid()
  if (info) {
    try {
      // Use process.kill which is cross-platform and safe
      process.kill(info.pid, 'SIGTERM')
    } catch { /* process may have already exited */ }
    // Clean up PID/Port files
    try { unlinkSync(MCP_PID_FILE) } catch { /* ignore */ }
    try { unlinkSync(MCP_PORT_FILE) } catch { /* ignore */ }
  }
  return { running: false }
}
