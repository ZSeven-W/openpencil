/**
 * Shared OpenCode client manager.
 * Reuses an existing server on port 4096; starts one on a random port as fallback.
 * Tracks spawned servers so they can be cleaned up on process exit.
 */
import { execSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join } from 'node:path'
import { homedir } from 'node:os'

const activeServers = new Set<{ close(): void }>()

// Clean up spawned OpenCode servers on process exit
function cleanup() {
  for (const server of activeServers) {
    try { server.close() } catch { /* ignore */ }
  }
  activeServers.clear()
}

process.on('beforeExit', cleanup)
process.on('SIGTERM', cleanup)
process.on('SIGINT', cleanup)

const isWindows = process.platform === 'win32'

/** Cached resolved binary path */
let _resolvedBinary: string | undefined | null = null

/** Resolve the opencode binary, with caching. */
function resolveOpencodeBinary(): string | undefined {
  if (_resolvedBinary !== null) return _resolvedBinary ?? undefined

  // PATH lookup
  try {
    const cmd = isWindows ? 'where opencode 2>nul' : 'which opencode 2>/dev/null'
    const result = execSync(cmd, { encoding: 'utf-8', timeout: 5000 }).trim().split(/\r?\n/)[0]?.trim()
    if (result && existsSync(result)) {
      _resolvedBinary = result
      return result
    }
  } catch { /* not on PATH */ }

  // Common install locations
  const home = homedir()
  const candidates = isWindows
    ? [
        join(process.env.APPDATA || '', 'npm', 'opencode.cmd'),
        join(process.env.NVM_SYMLINK || '', 'opencode.cmd'),
        join(process.env.FNM_MULTISHELL_PATH || '', 'opencode.cmd'),
        join(home, 'scoop', 'shims', 'opencode.exe'),
      ]
    : [
        join(home, '.opencode', 'bin', 'opencode'),
        join(home, '.npm-global', 'bin', 'opencode'),
        '/usr/local/bin/opencode',
        '/opt/homebrew/bin/opencode',
        join(home, '.local', 'bin', 'opencode'),
      ]

  for (const c of candidates) {
    if (c && existsSync(c)) {
      _resolvedBinary = c
      return c
    }
  }

  _resolvedBinary = undefined
  return undefined
}

export async function getOpencodeClient(binaryPath?: string) {
  const { createOpencodeClient, createOpencode } = await import('../opencode/index')

  // Try connecting to an existing server first
  try {
    const client = createOpencodeClient()
    await client.config.providers() // probe
    return { client, server: undefined }
  } catch {
    // No running server — start a temporary one on a random port
    const resolvedPath = binaryPath ?? resolveOpencodeBinary()
    const timeout = isWindows ? 15_000 : 5000
    const oc = await createOpencode({ port: 0, binaryPath: resolvedPath, timeout })
    activeServers.add(oc.server)
    return { client: oc.client, server: oc.server }
  }
}

export function releaseOpencodeServer(server: { close(): void } | undefined) {
  if (!server) return
  try { server.close() } catch { /* ignore */ }
  activeServers.delete(server)
}
