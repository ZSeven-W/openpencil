import { defineEventHandler, readBody, setResponseHeaders } from 'h3'
import { homedir } from 'node:os'
import { join, resolve } from 'node:path'
import { readFile, writeFile, mkdir } from 'node:fs/promises'
import { existsSync } from 'node:fs'

interface InstallBody {
  tool: string
  action: 'install' | 'uninstall'
}

interface InstallResult {
  success: boolean
  error?: string
  configPath?: string
}

const MCP_SERVER_NAME = 'openpencil'

/**
 * Resolve the absolute path to the compiled MCP server.
 * In dev: <project>/dist/mcp-server.cjs
 * In production (Electron): <resources>/mcp-server.cjs
 */
function resolveMcpServerPath(): string {
  // Try dist/ in the project root first (dev + compiled)
  const projectDist = resolve(process.cwd(), 'dist', 'mcp-server.cjs')
  if (existsSync(projectDist)) return projectDist
  // Fallback: try relative to this file
  const serverDist = resolve(__dirname, '..', '..', '..', 'dist', 'mcp-server.cjs')
  if (existsSync(serverDist)) return serverDist
  return projectDist // Return expected path even if not yet compiled
}

/** Config file locations and formats for each CLI tool. */
const CLI_CONFIGS: Record<
  string,
  {
    configPath: () => string
    read: (filePath: string) => Promise<Record<string, any>>
    write: (filePath: string, config: Record<string, any>) => Promise<void>
    install: (config: Record<string, any>, serverPath: string) => Record<string, any>
    uninstall: (config: Record<string, any>) => Record<string, any>
  }
> = {
  'claude-code': {
    configPath: () => join(homedir(), '.claude.json'),
    read: readJsonConfig,
    write: writeJsonConfig,
    install: (config, serverPath) => ({
      ...config,
      mcpServers: {
        ...(config.mcpServers ?? {}),
        [MCP_SERVER_NAME]: { command: 'node', args: [serverPath] },
      },
    }),
    uninstall: (config) => {
      const servers = { ...(config.mcpServers ?? {}) }
      delete servers[MCP_SERVER_NAME]
      return {
        ...config,
        mcpServers: Object.keys(servers).length > 0 ? servers : undefined,
      }
    },
  },
  'codex-cli': {
    configPath: () => join(homedir(), '.codex', 'config.json'),
    read: readJsonConfig,
    write: writeJsonConfig,
    install: (config, serverPath) => ({
      ...config,
      mcpServers: {
        ...(config.mcpServers ?? {}),
        [MCP_SERVER_NAME]: { command: 'node', args: [serverPath] },
      },
    }),
    uninstall: (config) => {
      const servers = { ...(config.mcpServers ?? {}) }
      delete servers[MCP_SERVER_NAME]
      return { ...config, mcpServers: Object.keys(servers).length > 0 ? servers : undefined }
    },
  },
  'gemini-cli': {
    configPath: () => join(homedir(), '.gemini', 'settings.json'),
    read: readJsonConfig,
    write: writeJsonConfig,
    install: (config, serverPath) => ({
      ...config,
      mcpServers: {
        ...(config.mcpServers ?? {}),
        [MCP_SERVER_NAME]: {
          command: 'node',
          args: [serverPath],
        },
      },
    }),
    uninstall: (config) => {
      const servers = { ...(config.mcpServers ?? {}) }
      delete servers[MCP_SERVER_NAME]
      return { ...config, mcpServers: Object.keys(servers).length > 0 ? servers : undefined }
    },
  },
  'opencode-cli': {
    configPath: () => join(homedir(), '.opencode', 'config.json'),
    read: readJsonConfig,
    write: writeJsonConfig,
    install: (config, serverPath) => ({
      ...config,
      mcpServers: {
        ...(config.mcpServers ?? {}),
        [MCP_SERVER_NAME]: { command: 'node', args: [serverPath] },
      },
    }),
    uninstall: (config) => {
      const servers = { ...(config.mcpServers ?? {}) }
      delete servers[MCP_SERVER_NAME]
      return { ...config, mcpServers: Object.keys(servers).length > 0 ? servers : undefined }
    },
  },
  'kiro-cli': {
    configPath: () => join(homedir(), '.kiro', 'settings.json'),
    read: readJsonConfig,
    write: writeJsonConfig,
    install: (config, serverPath) => ({
      ...config,
      mcpServers: {
        ...(config.mcpServers ?? {}),
        [MCP_SERVER_NAME]: { command: 'node', args: [serverPath] },
      },
    }),
    uninstall: (config) => {
      const servers = { ...(config.mcpServers ?? {}) }
      delete servers[MCP_SERVER_NAME]
      return { ...config, mcpServers: Object.keys(servers).length > 0 ? servers : undefined }
    },
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
    const serverPath = resolveMcpServerPath()

    const updated =
      body.action === 'install'
        ? cliConfig.install(config, serverPath)
        : cliConfig.uninstall(config)

    await cliConfig.write(configPath, updated)

    return { success: true, configPath } satisfies InstallResult
  } catch (err) {
    return {
      success: false,
      error: err instanceof Error ? err.message : String(err),
    } satisfies InstallResult
  }
})
