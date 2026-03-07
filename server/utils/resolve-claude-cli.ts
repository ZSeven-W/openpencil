import { execSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { homedir, platform } from 'node:os'
import { join } from 'node:path'

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
  // 1. Try PATH lookup
  try {
    const cmd = isWindows ? 'where claude' : 'which claude 2>/dev/null'
    const p = execSync(cmd, {
      encoding: 'utf-8',
      timeout: 3000,
    }).trim().split(/\r?\n/)[0] // `where` on Windows may return multiple lines
    if (p && existsSync(p)) return p
  } catch { /* not in PATH */ }

  // 2. Common install locations
  const candidates = isWindows
    ? [
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
    if (c && existsSync(c)) return c
  }

  return undefined
}
