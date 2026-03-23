import { defineEventHandler, readBody, setResponseHeaders } from 'h3'
import { homedir } from 'node:os'
import { join, resolve, dirname } from 'node:path'
import { readFile, writeFile, mkdir } from 'node:fs/promises'
import { existsSync } from 'node:fs'
import { execSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

// ESM-compatible __dirname polyfill
const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

const MCP_DEFAULT_PORT = 3100

interface InstallBody {
  tool: string
  action: 'install' | 'uninstall'
  transportMode?: 'stdio' | 'http' | 'both'
  httpPort?: number
}

interface InstallResult {
  success: boolean
  error?: string
  configPath?: string
  /** True when node was not found and HTTP URL fallback was used */
  fallbackHttp?: boolean
}

const MCP_SERVER_NAME = 'openpencil'

/**
 * Resolve the absolute path to the compiled MCP server.
 * In dev: <project>/dist/mcp-server.cjs
 * In production (Electron): <resources>/mcp-server.cjs
 */
function resolveMcpServerPath(): string {
  // Electron production: extraResources places it in resourcesPath
  const electronResources = process.env.ELECTRON_RESOURCES_PATH
  if (electronResources) {
    const electronPath = join(electronResources, 'mcp-server.cjs')
    if (existsSync(electronPath)) return electronPath
  }
  // Monorepo root: cwd may be apps/web (dev) or project root (Electron)
  // Walk up from cwd to find monorepo root (has package.json with workspaces)
  let root = process.cwd()
  for (let i = 0; i < 5; i++) {
    const candidate = join(root, 'out', 'mcp-server.cjs')
    if (existsSync(candidate)) return candidate
    const parent = dirname(root)
    if (parent === root) break
    root = parent
  }
  // Fallback: try relative to this file (Nitro bundles server code)
  const fromFile = resolve(__dirname, '..', '..', '..', 'out', 'mcp-server.cjs')
  if (existsSync(fromFile)) return fromFile
  // Return expected monorepo root path
  return join(root, 'out', 'mcp-server.cjs')
}

/**
 * Detect if `node` is available on the system.
 * Checks PATH first, then common install locations (the Nitro/Electron
 * process may run with a stripped PATH that doesn't include the user's
 * node installation).
 * Caches the result for the lifetime of the process.
 */
let _nodeAvailable: boolean | null = null
function isNodeAvailable(): boolean {
  if (_nodeAvailable !== null) return _nodeAvailable

  // Try PATH first
  try {
    execSync('node --version', { stdio: 'ignore', timeout: 5000 })
    _nodeAvailable = true
    return true
  } catch { /* not on PATH */ }

  // Check common absolute paths (macOS/Linux + Windows)
  const candidates = process.platform === 'win32'
    ? [
        join(process.env.ProgramFiles ?? 'C:\\Program Files', 'nodejs', 'node.exe'),
        join(process.env.LOCALAPPDATA ?? '', 'fnm_multishells', '**', 'node.exe'),
        join(homedir(), '.nvm', 'current', 'bin', 'node.exe'),
      ]
    : [
        '/usr/local/bin/node',
        '/usr/bin/node',
        join(homedir(), '.nvm', 'versions', 'node'),  // nvm directory
        '/opt/homebrew/bin/node',
      ]

  for (const p of candidates) {
    if (existsSync(p)) {
      _nodeAvailable = true
      return true
    }
  }

  _nodeAvailable = false
  return false
}

function buildMcpServerEntry(
  serverPath: string,
  transportMode: 'stdio' | 'http' | 'both' = 'stdio',
  httpPort = MCP_DEFAULT_PORT,
): { command: string; args: string[] } {
  switch (transportMode) {
    case 'http':
      return { command: 'node', args: [serverPath, '--http', '--port', String(httpPort)] }
    case 'both':
      return { command: 'node', args: [serverPath, '--http', '--port', String(httpPort), '--stdio'] }
    default:
      return { command: 'node', args: [serverPath] }
  }
}

/** Build an HTTP URL-based MCP server entry (no local node required). */
function buildMcpHttpUrlEntry(httpPort = MCP_DEFAULT_PORT): { type: 'http'; url: string } {
  return { type: 'http', url: `http://127.0.0.1:${httpPort}/mcp` }
}

/** Config file locations and formats for each CLI tool. */
interface CliConfigDef {
  configPath: () => string
  read: (filePath: string) => Promise<Record<string, any>>
  write: (filePath: string, config: Record<string, any>) => Promise<void>
}

function installMcpServer(
  config: Record<string, any>,
  serverPath: string,
  transportMode?: 'stdio' | 'http' | 'both',
  httpPort?: number,
): Record<string, any> {
  return {
    ...config,
    mcpServers: {
      ...(config.mcpServers ?? {}),
      [MCP_SERVER_NAME]: buildMcpServerEntry(serverPath, transportMode, httpPort),
    },
  }
}

/** Install MCP server using HTTP URL (for environments without node). */
function installMcpServerHttpUrl(
  config: Record<string, any>,
  httpPort?: number,
): Record<string, any> {
  return {
    ...config,
    mcpServers: {
      ...(config.mcpServers ?? {}),
      [MCP_SERVER_NAME]: buildMcpHttpUrlEntry(httpPort),
    },
  }
}

function uninstallMcpServer(config: Record<string, any>): Record<string, any> {
  const servers = { ...(config.mcpServers ?? {}) }
  delete servers[MCP_SERVER_NAME]
  return { ...config, mcpServers: Object.keys(servers).length > 0 ? servers : undefined }
}

const CLI_CONFIGS: Record<string, CliConfigDef> = {
  'claude-code': {
    configPath: () => join(homedir(), '.claude.json'),
    read: readJsonConfig,
    write: writeJsonConfig,
  },
  'codex-cli': {
    configPath: () => join(homedir(), '.codex', 'config.json'),
    read: readJsonConfig,
    write: writeJsonConfig,
  },
  'gemini-cli': {
    configPath: () => join(homedir(), '.gemini', 'settings.json'),
    read: readJsonConfig,
    write: writeJsonConfig,
  },
  'opencode-cli': {
    configPath: () => join(homedir(), '.opencode', 'config.json'),
    read: readJsonConfig,
    write: writeJsonConfig,
  },
  'kiro-cli': {
    configPath: () => join(homedir(), '.kiro', 'settings.json'),
    read: readJsonConfig,
    write: writeJsonConfig,
  },
  'copilot-cli': {
    configPath: () => join(homedir(), '.config', 'github-copilot', 'mcp.json'),
    read: readJsonConfig,
    write: writeJsonConfig,
  },
}

async function readJsonConfig(filePath: string): Promise<Record<string, any>> {
  try {
    const text = await readFile(filePath, 'utf-8')
    return JSON.parse(text)
  } catch {
    return {}
  }
}

async function writeJsonConfig(
  filePath: string,
  config: Record<string, any>,
): Promise<void> {
  const dir = join(filePath, '..')
  if (!existsSync(dir)) {
    await mkdir(dir, { recursive: true })
  }
  await writeFile(filePath, JSON.stringify(config, null, 2) + '\n', 'utf-8')
}

/**
 * POST /api/ai/mcp-install
 * Install or uninstall the openpencil MCP server into a CLI tool's config.
 */
export default defineEventHandler(async (event) => {
  const body = await readBody<InstallBody>(event)
  setResponseHeaders(event, { 'Content-Type': 'application/json' })

  if (!body?.tool || !body?.action) {
    return { success: false, error: 'Missing tool or action field' } satisfies InstallResult
  }

  const cliConfig = CLI_CONFIGS[body.tool]
  if (!cliConfig) {
    return { success: false, error: `Unknown CLI tool: ${body.tool}` } satisfies InstallResult
  }

  try {
    const configPath = cliConfig.configPath()
    const config = await cliConfig.read(configPath)

    let updated: Record<string, any>
    let fallbackHttp = false

    if (body.action === 'uninstall') {
      updated = uninstallMcpServer(config)
    } else if (!isNodeAvailable()) {
      // No node on this machine — fall back to HTTP URL config
      // and ensure the MCP HTTP server is running
      const httpPort = body.httpPort ?? MCP_DEFAULT_PORT
      updated = installMcpServerHttpUrl(config, httpPort)
      fallbackHttp = true

      // Auto-start the MCP HTTP server so the URL is reachable
      try {
        const { startMcpHttpServer } = await import('../../utils/mcp-server-manager')
        startMcpHttpServer(httpPort)
      } catch {
        // Non-fatal: server may already be running or will be started manually
      }
    } else {
      const serverPath = resolveMcpServerPath()
      updated = installMcpServer(config, serverPath, body.transportMode, body.httpPort)
    }

    await cliConfig.write(configPath, updated)

    return { success: true, configPath, ...(fallbackHttp ? { fallbackHttp } : {}) } satisfies InstallResult
  } catch (err) {
    return {
      success: false,
      error: err instanceof Error ? err.message : String(err),
    } satisfies InstallResult
  }
})
