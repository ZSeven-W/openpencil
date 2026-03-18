import { execSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { homedir, platform } from 'node:os'
import { join } from 'node:path'
import { serverLog } from './server-logger'

const isWindows = platform() === 'win32'

/**
 * Resolve the absolute path to the standalone `claude` binary.
 *
 * When Nitro bundles @anthropic-ai/claude-agent-sdk, the SDK's internal
 * `import.meta.url`-based resolution to find its own `cli.js` breaks.
 * Instead we locate the standalone native binary and pass it via
 * `pathToClaudeCodeExecutable` — the SDK detects non-.js paths as native
 * binaries and spawns them directly (no `node` wrapper needed).
 */
export function resolveClaudeCli(): string | undefined {
  serverLog.info(`[resolve-claude-cli] platform=${platform()}, isWindows=${isWindows}`)

  // 1. Try PATH lookup
  try {
    const cmd = isWindows ? 'where claude 2>nul' : 'which claude 2>/dev/null'
    serverLog.info(`[resolve-claude-cli] PATH lookup: ${cmd}`)
    const raw = execSync(cmd, {
      encoding: 'utf-8',
      timeout: 3000,
    }).trim()
    const p = raw.split(/\r?\n/)[0] // `where` on Windows may return multiple lines
    serverLog.info(`[resolve-claude-cli] PATH lookup result: "${p}" (exists=${p ? existsSync(p) : false})`)
    if (p && existsSync(p)) return p
  } catch (err) {
    serverLog.info(`[resolve-claude-cli] PATH lookup failed: ${err instanceof Error ? err.message : err}`)
  }

  // 2. Try `npm prefix -g` to find actual npm global bin directory
  //    On Windows, must use `npm.cmd` since Electron spawns cmd.exe
  if (isWindows) {
    try {
      serverLog.info('[resolve-claude-cli] trying npm.cmd prefix -g')
      const prefix = execSync('npm.cmd prefix -g', {
        encoding: 'utf-8',
        timeout: 5000,
      }).trim()
      serverLog.info(`[resolve-claude-cli] npm global prefix: "${prefix}"`)
      if (prefix) {
        const bin = join(prefix, 'claude.cmd')
        serverLog.info(`[resolve-claude-cli] checking npm global bin: "${bin}" (exists=${existsSync(bin)})`)
        if (existsSync(bin)) return bin
      }
    } catch (err) {
      serverLog.info(`[resolve-claude-cli] npm prefix -g failed: ${err instanceof Error ? err.message : err}`)
    }
  }

  // 3. Common install locations
  const candidates = isWindows
    ? [
        // npm global (npm install -g creates .cmd wrappers here)
        join(process.env.APPDATA || '', 'npm', 'claude.cmd'),
        // nvm-windows / fnm
        join(process.env.NVM_SYMLINK || '', 'claude.cmd'),
        join(process.env.FNM_MULTISHELL_PATH || '', 'claude.cmd'),
        // Native .exe install locations
        join(process.env.LOCALAPPDATA || '', 'Programs', 'claude-code', 'claude.exe'),
        join(process.env.LOCALAPPDATA || '', 'Microsoft', 'WinGet', 'Links', 'claude.exe'),
        join(homedir(), '.claude', 'local', 'claude.exe'),
        join(homedir(), 'AppData', 'Local', 'Programs', 'claude-code', 'claude.exe'),
      ]
    : [
        join(homedir(), '.local', 'bin', 'claude'),
        '/usr/local/bin/claude',
        '/opt/homebrew/bin/claude',
      ]
  for (const c of candidates) {
    const exists = c ? existsSync(c) : false
    serverLog.info(`[resolve-claude-cli] candidate: "${c}" (exists=${exists})`)
    if (c && exists) return c
  }

  serverLog.warn('[resolve-claude-cli] no claude binary found')
  return undefined
}
