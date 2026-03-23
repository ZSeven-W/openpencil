import { spawn } from 'node:child_process'
import { mkdirSync, readFileSync, writeFileSync, existsSync, appendFileSync } from 'node:fs'
import { homedir, tmpdir, platform } from 'node:os'
import { join } from 'node:path'

const IS_WIN = platform() === 'win32'

type EnvLike = Record<string, string | undefined>

interface ClaudeSettings {
  env?: Record<string, unknown>
}

function normalizeEnvValue(key: string, value: unknown): string | undefined {
  if (value == null) return undefined
  if (typeof value === 'string') {
    // Filter out empty strings - they cause issues
    if (value.trim() === '') return undefined
    return value
  }
  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value)
  }
  // ANTHROPIC_CUSTOM_HEADERS can be an object in settings.json — serialize it.
  // Other object values are skipped to prevent "Invalid header name" errors.
  if (typeof value === 'object') {
    if (key === 'ANTHROPIC_CUSTOM_HEADERS') {
      try { return JSON.stringify(value) } catch { return undefined }
    }
    return undefined
  }
  return undefined
}

function readSingleSettingsFile(filePath: string): EnvLike {
  try {
    const raw = readFileSync(filePath, 'utf-8')
    const parsed = JSON.parse(raw) as ClaudeSettings
    if (!parsed.env || typeof parsed.env !== 'object') return {}

    const env: EnvLike = {}
    for (const [key, value] of Object.entries(parsed.env)) {
      const normalized = normalizeEnvValue(key, value)
      if (normalized !== undefined) {
        env[key] = normalized
      }
    }
    return env
  } catch {
    return {}
  }
}

/**
 * Read env from ~/.claude/settings.json and ~/.claude/settings.local.json.
 * Local settings take priority (same as Claude Code's own precedence).
 */
function readClaudeSettingsEnv(): EnvLike {
  const claudeDir = join(homedir(), '.claude')
  const base = readSingleSettingsFile(join(claudeDir, 'settings.json'))
  const local = readSingleSettingsFile(join(claudeDir, 'settings.local.json'))
  return { ...base, ...local }
}

/**
 * Validate if a string is valid JSON (for ANTHROPIC_CUSTOM_HEADERS).
 */
function isValidJson(str: string): boolean {
  try {
    JSON.parse(str)
    return true
  } catch {
    return false
  }
}

/**
 * On Windows, Claude Code SDK may fail with EPERM when writing to ~/.claude.json
 * or ~/.claude/ config files. Ensure the directory and config file exist and are writable.
 */
function ensureClaudeConfigWritable(): void {
  if (!IS_WIN) return
  try {
    const claudeDir = join(homedir(), '.claude')
    mkdirSync(claudeDir, { recursive: true })
    // Ensure .claude.json exists — Claude SDK crashes if it can't write/lock it
    const configFile = join(homedir(), '.claude.json')
    if (!existsSync(configFile)) {
      writeFileSync(configFile, '{}', 'utf-8')
    }
    // Ensure credentials.json exists — SDK may crash trying to read/write it
    const credFile = join(claudeDir, 'credentials.json')
    if (!existsSync(credFile)) {
      writeFileSync(credFile, '{}', 'utf-8')
    }
    // Ensure statsig/ cache dir exists — SDK crashes writing feature gate cache
    const statsigDir = join(claudeDir, 'statsig')
    mkdirSync(statsigDir, { recursive: true })
  } catch {
    // Best effort — if we can't fix it, the SDK error hint will guide the user
  }
}

/**
 * Build env passed to Claude Agent SDK.
 * Priority: current process env > ~/.claude/settings.json env.
 */
export function buildClaudeAgentEnv(): EnvLike {
  // On Windows, pre-create config files to avoid EPERM errors
  ensureClaudeConfigWritable()

  const fromSettings = readClaudeSettingsEnv()
  const fromProcess = process.env as EnvLike

  const merged: EnvLike = {
    ...fromSettings,
    ...fromProcess,
  }

  // Validate ANTHROPIC_CUSTOM_HEADERS if it exists - must be valid JSON
  // If invalid, delete it to prevent "Invalid header name" errors
  if (merged.ANTHROPIC_CUSTOM_HEADERS) {
    if (!isValidJson(merged.ANTHROPIC_CUSTOM_HEADERS)) {
      delete merged.ANTHROPIC_CUSTOM_HEADERS
    }
  }

  // Compatibility: use ANTHROPIC_AUTH_TOKEN as ANTHROPIC_API_KEY if no API key is set
  const authToken = merged.ANTHROPIC_AUTH_TOKEN
  if (authToken && !merged.ANTHROPIC_API_KEY) {
    merged.ANTHROPIC_API_KEY = authToken
  }

  // Running inside Claude terminal can break nested Claude invocations.
  delete merged.CLAUDECODE

  // Remove Electron-specific env vars that may confuse spawned CLI processes
  delete merged.ELECTRON_RUN_AS_NODE
  delete merged.ELECTRON_RESOURCES_PATH
  delete merged.CHROME_CRASHPAD_PIPE_NAME

  // Enable Agent SDK debug stderr so we can capture CLI crash diagnostics.
  // Without this, the SDK sets stderr to "ignore" and crash output is lost.
  if (!merged.DEBUG_CLAUDE_AGENT_SDK) {
    merged.DEBUG_CLAUDE_AGENT_SDK = '1'
  }

  // Apply NODE_TLS_REJECT_UNAUTHORIZED to the current process as well,
  // so Node.js HTTP/TLS in this process (used by the SDK internals) respects it.
  if (merged.NODE_TLS_REJECT_UNAUTHORIZED && !process.env.NODE_TLS_REJECT_UNAUTHORIZED) {
    process.env.NODE_TLS_REJECT_UNAUTHORIZED = merged.NODE_TLS_REJECT_UNAUTHORIZED
  }

  if (IS_WIN) {
    // Redirect Claude debug output to temp to avoid write permission issues
    if (!merged.CLAUDE_DEBUG_FILE) {
      const debugPath = getClaudeAgentDebugFilePath()
      if (debugPath) merged.CLAUDE_DEBUG_FILE = debugPath
    }
    // Set CLAUDE_CONFIG_DIR to a writable temp location as fallback
    // if the default ~/.claude directory is not writable (common in Windows Electron)
    if (!merged.CLAUDE_CONFIG_DIR) {
      try {
        const fallbackDir = join(tmpdir(), 'openpencil-claude-config')
        mkdirSync(fallbackDir, { recursive: true })
        // Only use fallback if we can't write to the default location
        const defaultDir = join(homedir(), '.claude')
        const testFile = join(defaultDir, '.write-test')
        try {
          writeFileSync(testFile, '', 'utf-8')
          const { unlinkSync } = require('node:fs')
          unlinkSync(testFile)
        } catch {
          // Default dir is not writable — use fallback
          merged.CLAUDE_CONFIG_DIR = fallbackDir
        }
      } catch { /* ignore */ }
    }
  }

  return merged
}

/**
 * Force Claude CLI debug output into a writable temp location.
 * This avoids crashes in restricted environments where ~/.claude/debug is not writable.
 */
export function getClaudeAgentDebugFilePath(): string | undefined {
  try {
    const dir = join(tmpdir(), 'openpencil-claude-debug')
    mkdirSync(dir, { recursive: true })
    return join(dir, 'claude-agent.log')
  } catch {
    return undefined
  }
}

/**
 * Custom spawnClaudeCodeProcess for Windows.
 * On Windows, npm-installed CLIs are .cmd/.ps1 scripts that can't be spawned
 * directly without a shell.
 *
 * - `.cmd` files: use `cmd.exe /c` (PowerShell can't run .cmd directly)
 * - `.ps1` files: use `powershell.exe`
 * - `.exe` files: spawned directly without shell
 * - Others: use `cmd.exe /c` as safe default
 *
 * Also captures stderr to the debug file — when Claude Code crashes early,
 * the debug file may be empty but stderr often contains the root cause.
 */
export function buildSpawnClaudeCodeProcess() {
  if (process.platform !== 'win32') return undefined
  return (options: { command: string; args: string[]; cwd?: string; env: Record<string, string | undefined>; signal: AbortSignal }) => {
    const cmd = options.command
    const isPowerShell = cmd.endsWith('.ps1')

    let child
    if (isPowerShell) {
      // For .ps1 scripts, invoke via PowerShell
      const psArgs = ['-ExecutionPolicy', 'Bypass', '-File', cmd, ...options.args]
      child = spawn('powershell.exe', psArgs, {
        cwd: options.cwd,
        env: options.env as NodeJS.ProcessEnv,
        signal: options.signal,
        stdio: ['pipe', 'pipe', 'pipe'],
        windowsHide: true,
      })
    } else if (cmd.endsWith('.exe')) {
      // .exe files can be spawned directly without shell
      child = spawn(cmd, options.args, {
        cwd: options.cwd,
        env: options.env as NodeJS.ProcessEnv,
        signal: options.signal,
        stdio: ['pipe', 'pipe', 'pipe'],
        windowsHide: true,
      })
    } else {
      // For .cmd or extensionless binaries, use shell
      child = spawn(cmd, options.args, {
        cwd: options.cwd,
        env: options.env as NodeJS.ProcessEnv,
        signal: options.signal,
        stdio: ['pipe', 'pipe', 'pipe'],
        shell: true,
        windowsHide: true,
      })
    }

    // Capture stderr to debug file — helps diagnose crashes where the process
    // exits before writing anything to the debug log
    const stderrChunks: Buffer[] = []
    child.stderr?.on('data', (chunk: Buffer) => { stderrChunks.push(chunk) })
    child.on('exit', (code) => {
      if (code !== 0 && stderrChunks.length > 0) {
        const stderr = Buffer.concat(stderrChunks).toString('utf-8').trim()
        if (stderr) {
          const debugPath = getClaudeAgentDebugFilePath()
          if (debugPath) {
            try {
              appendFileSync(debugPath, `\n[stderr exit=${code}] ${stderr}\n`)
            } catch { /* best effort */ }
          }
        }
      }
    })

    return child
  }
}
