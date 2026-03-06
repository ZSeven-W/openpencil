import { execSync } from 'node:child_process'

/** Resolve the standalone copilot CLI binary path to avoid Bun's node:sqlite issue */
export function resolveCopilotCli(): string | undefined {
  try {
    const path = execSync('which copilot 2>/dev/null', { encoding: 'utf-8', timeout: 5000 }).trim()
    return path || undefined
  } catch {
    return undefined
  }
}
