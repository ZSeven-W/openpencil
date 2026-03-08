import { execSync } from 'node:child_process'

const isWindows = process.platform === 'win32'

/** Resolve the standalone copilot CLI binary path to avoid Bun's node:sqlite issue */
export function resolveCopilotCli(): string | undefined {
  try {
    const cmd = isWindows ? 'where copilot 2>nul' : 'which copilot 2>/dev/null'
    const result = execSync(cmd, { encoding: 'utf-8', timeout: 5000 }).trim()
    // `where` on Windows may return multiple lines
    const path = result.split(/\r?\n/)[0]?.trim()
    return path || undefined
  } catch {
    return undefined
  }
}
