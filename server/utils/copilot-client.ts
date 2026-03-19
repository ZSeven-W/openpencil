import { execSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join } from 'node:path'
import { serverLog } from './server-logger'

const isWindows = process.platform === 'win32'

/** Windows npm global installs may create .cmd or .ps1 wrappers — try both */
function winNpmCandidates(dir: string, name: string): string[] {
  return [join(dir, `${name}.cmd`), join(dir, `${name}.ps1`)]
}

/** On Windows, `where` may return an extensionless shell script — prefer .cmd/.ps1 */
function resolveWinExtension(binPath: string): string {
  if (!isWindows) return binPath
  if (/\.(cmd|ps1|exe)$/i.test(binPath)) return binPath
  for (const ext of ['.cmd', '.ps1']) {
    if (existsSync(binPath + ext)) return binPath + ext
  }
  return binPath
}

/** Resolve the standalone copilot CLI binary path to avoid Bun's node:sqlite issue */
export function resolveCopilotCli(): string | undefined {
  serverLog.info(`[resolve-copilot] platform=${process.platform}, isWindows=${isWindows}`)

  // 1. Try PATH lookup
  try {
    const cmd = isWindows ? 'where copilot 2>nul' : 'which copilot 2>/dev/null'
    serverLog.info(`[resolve-copilot] PATH lookup: ${cmd}`)
    const result = execSync(cmd, { encoding: 'utf-8', timeout: 5000 }).trim()
    // `where` on Windows may return multiple lines
    const path = result.split(/\r?\n/)[0]?.trim()
    serverLog.info(`[resolve-copilot] PATH result: "${path}" (exists=${path ? existsSync(path) : false})`)
    if (path && existsSync(path)) return resolveWinExtension(path)
  } catch (err) {
    serverLog.info(`[resolve-copilot] PATH lookup failed: ${err instanceof Error ? err.message : err}`)
  }

  // 2. Try `npm prefix -g` on Windows (npm install -g creates .cmd wrappers)
  if (isWindows) {
    try {
      serverLog.info('[resolve-copilot] trying npm.cmd prefix -g')
      const prefix = execSync('npm.cmd prefix -g', {
        encoding: 'utf-8',
        timeout: 5000,
      }).trim()
      serverLog.info(`[resolve-copilot] npm global prefix: "${prefix}"`)
      if (prefix) {
        for (const bin of winNpmCandidates(prefix, 'copilot')) {
          serverLog.info(`[resolve-copilot] npm global bin: "${bin}" (exists=${existsSync(bin)})`)
          if (existsSync(bin)) return bin
        }
      }
    } catch (err) {
      serverLog.info(`[resolve-copilot] npm prefix -g failed: ${err instanceof Error ? err.message : err}`)
    }
  }

  // 3. Common install locations
  if (isWindows) {
    const candidates = [
      // npm global (.cmd + .ps1)
      ...winNpmCandidates(join(process.env.APPDATA || '', 'npm'), 'copilot'),
      // nvm-windows / fnm
      ...winNpmCandidates(join(process.env.NVM_SYMLINK || ''), 'copilot'),
      ...winNpmCandidates(join(process.env.FNM_MULTISHELL_PATH || ''), 'copilot'),
      // winget / native
      join(process.env.LOCALAPPDATA || '', 'Microsoft', 'WinGet', 'Links', 'copilot.exe'),
    ]
    for (const c of candidates) {
      const exists = c ? existsSync(c) : false
      serverLog.info(`[resolve-copilot] candidate: "${c}" (exists=${exists})`)
      if (c && exists) return c
    }
  }

  serverLog.warn('[resolve-copilot] no copilot binary found')
  return undefined
}
