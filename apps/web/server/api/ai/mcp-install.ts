import { defineEventHandler, readBody, setResponseHeaders } from 'h3';
import { homedir } from 'node:os';
import { join, resolve, dirname } from 'node:path';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { existsSync, readdirSync, statSync } from 'node:fs';
import { execSync, execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

// ESM-compatible __dirname polyfill
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const MCP_DEFAULT_PORT = 3100;

interface InstallBody {
  tool: string;
  action: 'install' | 'uninstall';
  transportMode?: 'stdio' | 'http' | 'both';
  httpPort?: number;
}

interface InstallResult {
  success: boolean;
  error?: string;
  configPath?: string;
  /** True when node was not found and HTTP URL fallback was used */
  fallbackHttp?: boolean;
}

const MCP_SERVER_NAME = 'openpencil';
const CODEX_CONFIG_PATH = join(homedir(), '.codex', 'config.toml');

/**
 * Resolve the absolute path to the compiled MCP server.
 * In dev: <project>/dist/mcp-server.cjs
 * In production (Electron): <resources>/mcp-server.cjs
 */
function resolveMcpServerPath(): string {
  // Electron production: extraResources places it in resourcesPath
  const electronResources = process.env.ELECTRON_RESOURCES_PATH;
  if (electronResources) {
    const electronPath = join(electronResources, 'mcp-server.cjs');
    if (existsSync(electronPath)) return electronPath;
  }
  // Monorepo root: cwd may be apps/web (dev) or project root (Electron)
  // Walk up from cwd to find monorepo root (has package.json with workspaces)
  let root = process.cwd();
  for (let i = 0; i < 5; i++) {
    const candidate = join(root, 'out', 'mcp-server.cjs');
    if (existsSync(candidate)) return candidate;
    const parent = dirname(root);
    if (parent === root) break;
    root = parent;
  }
  // Fallback: try relative to this file (Nitro bundles server code)
  const fromFile = resolve(__dirname, '..', '..', '..', 'out', 'mcp-server.cjs');
  if (existsSync(fromFile)) return fromFile;
  // Return expected monorepo root path
  return join(root, 'out', 'mcp-server.cjs');
}

/**
 * Detect if `node` is available on the system.
 * Checks PATH first, then common install locations (the Nitro/Electron
 * process may run with a stripped PATH that doesn't include the user's
 * node installation).
 * Caches the result for the lifetime of the process.
 */
let _nodeAvailable: boolean | null = null;
let _nodeCommand: string | null = null;

function nodeCandidates(): string[] {
  if (process.platform === 'win32') {
    return [
      'node.exe',
      join(process.env.ProgramFiles ?? 'C:\\Program Files', 'nodejs', 'node.exe'),
      join(process.env.LOCALAPPDATA ?? '', 'fnm_multishells', '**', 'node.exe'),
      join(homedir(), '.nvm', 'current', 'bin', 'node.exe'),
    ];
  }

  const candidates = ['node', '/usr/local/bin/node', '/usr/bin/node', '/opt/homebrew/bin/node'];

  // NVM: resolve the active node version via the symlink or by reading
  // .nvm/alias/default, then constructing the versioned bin path.
  // The old path (`.nvm/versions/node`) is a directory, not a binary —
  // existsSync would return true but executing it gives "Permission denied".
  const nvmDir = join(homedir(), '.nvm');
  const nvmCurrent = join(nvmDir, 'current', 'bin', 'node');
  candidates.push(nvmCurrent);

  // Fallback: if NVM_DIR/current doesn't exist, find the highest installed version
  if (!existsSync(nvmCurrent)) {
    const versionsDir = join(nvmDir, 'versions', 'node');
    if (existsSync(versionsDir)) {
      try {
        const versions = readdirSync(versionsDir)
          .filter((d) => d.startsWith('v'))
          .sort();
        if (versions.length > 0) {
          candidates.push(join(versionsDir, versions[versions.length - 1], 'bin', 'node'));
        }
      } catch {
        /* ignore */
      }
    }
  }

  return candidates;
}

function isNodeAvailable(): boolean {
  if (_nodeAvailable !== null) return _nodeAvailable;

  // Try PATH first
  try {
    const whichCmd = process.platform === 'win32' ? 'where node 2>nul' : 'which node 2>/dev/null';
    const resolved = execSync(whichCmd, { encoding: 'utf-8', timeout: 5000 })
      .trim()
      .split(/\r?\n/)[0]
      ?.trim();
    _nodeCommand = resolved || 'node';
    _nodeAvailable = true;
    return true;
  } catch {
    /* not on PATH */
  }

  // Check common absolute paths (macOS/Linux + Windows).
  // Must verify the path is a file, not a directory — existsSync returns
  // true for directories, which caused the NVM versions dir to be treated
  // as a node binary.
  for (const p of nodeCandidates().slice(1)) {
    try {
      if (existsSync(p) && statSync(p).isFile()) {
        _nodeCommand = p;
        _nodeAvailable = true;
        return true;
      }
    } catch {
      /* ignore stat errors */
    }
  }

  _nodeAvailable = false;
  return false;
}

function resolveNodeCommand(): string {
  if (isNodeAvailable()) return _nodeCommand ?? 'node';
  throw new Error('Node.js not found');
}

function buildMcpServerEntry(
  serverPath: string,
  transportMode: 'stdio' | 'http' | 'both' = 'stdio',
  httpPort = MCP_DEFAULT_PORT,
): { command: string; args: string[] } {
  switch (transportMode) {
    case 'http':
      return { command: 'node', args: [serverPath, '--http', '--port', String(httpPort)] };
    case 'both':
      return {
        command: 'node',
        args: [serverPath, '--http', '--port', String(httpPort), '--stdio'],
      };
    default:
      return { command: 'node', args: [serverPath] };
  }
}

/** Build an HTTP URL-based MCP server entry (no local node required). */
function buildMcpHttpUrlEntry(httpPort = MCP_DEFAULT_PORT): { type: 'http'; url: string } {
  return { type: 'http', url: `http://127.0.0.1:${httpPort}/mcp` };
}

/** Config file locations and formats for each CLI tool. */
interface CliConfigDef {
  configPath: () => string;
  read: (filePath: string) => Promise<Record<string, any>>;
  write: (filePath: string, config: Record<string, any>) => Promise<void>;
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
      ...config.mcpServers,
      [MCP_SERVER_NAME]: buildMcpServerEntry(serverPath, transportMode, httpPort),
    },
  };
}

/** Install MCP server using HTTP URL (for environments without node). */
function installMcpServerHttpUrl(
  config: Record<string, any>,
  httpPort?: number,
): Record<string, any> {
  return {
    ...config,
    mcpServers: {
      ...config.mcpServers,
      [MCP_SERVER_NAME]: buildMcpHttpUrlEntry(httpPort),
    },
  };
}

function uninstallMcpServer(config: Record<string, any>): Record<string, any> {
  const servers = { ...config.mcpServers };
  delete servers[MCP_SERVER_NAME];
  return { ...config, mcpServers: Object.keys(servers).length > 0 ? servers : undefined };
}

const CLI_CONFIGS: Record<string, CliConfigDef> = {
  'claude-code': {
    configPath: () => join(homedir(), '.claude.json'),
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
};

async function readJsonConfig(filePath: string): Promise<Record<string, any>> {
  try {
    const text = await readFile(filePath, 'utf-8');
    return JSON.parse(text);
  } catch {
    return {};
  }
}

async function writeJsonConfig(filePath: string, config: Record<string, any>): Promise<void> {
  const dir = join(filePath, '..');
  if (!existsSync(dir)) {
    await mkdir(dir, { recursive: true });
  }
  await writeFile(filePath, JSON.stringify(config, null, 2) + '\n', 'utf-8');
}

/**
 * Run the codex CLI with args, handling Windows shim spawning.
 *
 * Since Node 18.20/20.12 (CVE-2024-27980), execFileSync refuses to spawn
 * .cmd/.bat files directly (throws EINVAL). On Windows we route through the
 * shell so PATHEXT resolves whichever shim exists (codex.exe / codex.cmd /
 * codex.ps1). Args here are internal (no user input) — we just quote paths
 * that contain spaces.
 */
function runCodex(args: string[]): void {
  if (process.platform === 'win32') {
    const cmdLine = ['codex', ...args].map(quoteWinArg).join(' ');
    execSync(cmdLine, { encoding: 'utf-8', timeout: 15_000, stdio: 'pipe' });
    return;
  }
  execFileSync('codex', args, {
    encoding: 'utf-8',
    timeout: 15_000,
    stdio: 'pipe',
  });
}

function quoteWinArg(arg: string): string {
  if (arg.length === 0) return '""';
  if (!/[\s"&|<>^%]/.test(arg)) return arg;
  return `"${arg.replace(/"/g, '""')}"`;
}

async function installCodexMcp(
  transportMode?: 'stdio' | 'http' | 'both',
  httpPort?: number,
): Promise<{ configPath: string; fallbackHttp: boolean }> {
  const serverPath = resolveMcpServerPath();
  const port = httpPort ?? MCP_DEFAULT_PORT;
  const useHttp = transportMode === 'http' || !isNodeAvailable();

  try {
    uninstallCodexMcp();
  } catch {
    // Ignore missing-entry cleanup failures; install below is the real operation.
  }

  if (useHttp) {
    try {
      const { startMcpHttpServer } = await import('../../utils/mcp-server-manager');
      startMcpHttpServer(port);
    } catch {
      // Non-fatal: server may already be running or will be started manually
    }

    runCodex(['mcp', 'add', MCP_SERVER_NAME, '--url', `http://127.0.0.1:${port}/mcp`]);
    return { configPath: CODEX_CONFIG_PATH, fallbackHttp: true };
  }

  runCodex(['mcp', 'add', MCP_SERVER_NAME, '--', resolveNodeCommand(), serverPath]);
  return { configPath: CODEX_CONFIG_PATH, fallbackHttp: false };
}

function uninstallCodexMcp(): { configPath: string } {
  runCodex(['mcp', 'remove', MCP_SERVER_NAME]);
  return { configPath: CODEX_CONFIG_PATH };
}

/**
 * POST /api/ai/mcp-install
 * Install or uninstall the openpencil MCP server into a CLI tool's config.
 */
export default defineEventHandler(async (event) => {
  const body = await readBody<InstallBody>(event);
  setResponseHeaders(event, { 'Content-Type': 'application/json' });

  if (!body?.tool || !body?.action) {
    return { success: false, error: 'Missing tool or action field' } satisfies InstallResult;
  }

  // Codex CLI uses its own `codex mcp add/remove` commands (writes ~/.codex/config.toml)
  if (body.tool === 'codex-cli') {
    try {
      const result =
        body.action === 'uninstall'
          ? uninstallCodexMcp()
          : await installCodexMcp(body.transportMode, body.httpPort);
      return {
        success: true,
        configPath: result.configPath,
        ...('fallbackHttp' in result && result.fallbackHttp ? { fallbackHttp: true } : {}),
      } satisfies InstallResult;
    } catch (err) {
      return {
        success: false,
        error: err instanceof Error ? err.message : String(err),
      } satisfies InstallResult;
    }
  }

  const cliConfig = CLI_CONFIGS[body.tool];
  if (!cliConfig) {
    return { success: false, error: `Unknown CLI tool: ${body.tool}` } satisfies InstallResult;
  }

  try {
    const configPath = cliConfig.configPath();
    const config = await cliConfig.read(configPath);

    let updated: Record<string, any>;
    let fallbackHttp = false;

    if (body.action === 'uninstall') {
      updated = uninstallMcpServer(config);
    } else if (!isNodeAvailable()) {
      // No node on this machine — fall back to HTTP URL config
      // and ensure the MCP HTTP server is running
      const httpPort = body.httpPort ?? MCP_DEFAULT_PORT;
      updated = installMcpServerHttpUrl(config, httpPort);
      fallbackHttp = true;

      // Auto-start the MCP HTTP server so the URL is reachable
      try {
        const { startMcpHttpServer } = await import('../../utils/mcp-server-manager');
        startMcpHttpServer(httpPort);
      } catch {
        // Non-fatal: server may already be running or will be started manually
      }
    } else {
      const serverPath = resolveMcpServerPath();
      updated = installMcpServer(config, serverPath, body.transportMode, body.httpPort);
    }

    await cliConfig.write(configPath, updated);

    return {
      success: true,
      configPath,
      ...(fallbackHttp ? { fallbackHttp } : {}),
    } satisfies InstallResult;
  } catch (err) {
    return {
      success: false,
      error: err instanceof Error ? err.message : String(err),
    } satisfies InstallResult;
  }
});
