import { defineEventHandler, readBody, setResponseHeaders } from 'h3'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import type { GroupedModel } from '../../../src/types/agent-settings'
import { resolveClaudeCli } from '../../utils/resolve-claude-cli'
import { serverLog } from '../../utils/server-logger'
import {
  buildClaudeAgentEnv,
  buildSpawnClaudeCodeProcess,
  getClaudeAgentDebugFilePath,
} from '../../utils/resolve-claude-agent-env'

/** Windows npm global installs may create .cmd or .ps1 wrappers — try both */
function winNpmCandidates(dir: string, name: string): string[] {
  return [join(dir, `${name}.cmd`), join(dir, `${name}.ps1`)]
}

/**
 * On Windows, `where` may return an extensionless Unix shell script (e.g. `…/npm/opencode`).
 * This file exists but can't be executed. Prefer `.cmd` or `.ps1` wrapper at the same location.
 */
function resolveWinExtension(binPath: string): string {
  if (process.platform !== 'win32') return binPath
  // Already has a usable extension
  if (/\.(cmd|ps1|exe)$/i.test(binPath)) return binPath
  // Try .cmd then .ps1
  for (const ext of ['.cmd', '.ps1']) {
    if (existsSync(binPath + ext)) return binPath + ext
  }
  return binPath
}

/** Build a shell command to invoke a resolved binary (handles .ps1 on Windows) */
function buildExecCmd(binPath: string, args: string): string {
  if (binPath.endsWith('.ps1')) {
    return `powershell -ExecutionPolicy Bypass -File "${binPath}" ${args}`
  }
  return `"${binPath}" ${args}`
}

interface ConnectBody {
  agent: 'claude-code' | 'codex-cli' | 'opencode' | 'copilot' | 'gemini-cli'
}

interface ConnectResult {
  connected: boolean
  models: GroupedModel[]
  error?: string
  warning?: string
  notInstalled?: boolean
  /** Human-readable connection status, e.g. "Connected via API key" */
  connectionInfo?: string
  /** Config file path for the hint (client renders localized text) */
  hintPath?: string
}

/**
 * POST /api/ai/connect-agent
 * Actively connects to a local CLI tool and fetches its supported models.
 */
export default defineEventHandler(async (event) => {
  const body = await readBody<ConnectBody>(event)
  setResponseHeaders(event, { 'Content-Type': 'application/json' })

  if (!body?.agent) {
    return { connected: false, models: [], error: 'Missing agent field' } satisfies ConnectResult
  }

  if (body.agent === 'claude-code') {
    return connectClaudeCode()
  }

  if (body.agent === 'codex-cli') {
    return connectCodexCli()
  }

  if (body.agent === 'opencode') {
    return connectOpenCode()
  }

  if (body.agent === 'copilot') {
    return connectCopilot()
  }

  if (body.agent === 'gemini-cli') {
    return connectGeminiCli()
  }

  return { connected: false, models: [], error: `Unknown agent: ${body.agent}` } satisfies ConnectResult
})

/**
 * Fallback models when supportedModels() fails.
 * Used with third-party API proxies (e.g. Claude Router) that don't support
 * the model-listing endpoint. Covers common model IDs routers typically expose.
 */
const FALLBACK_CLAUDE_MODELS: GroupedModel[] = [
  { value: 'claude-sonnet-4-6', displayName: 'Claude Sonnet 4.6', description: '', provider: 'anthropic' },
  { value: 'claude-opus-4-6', displayName: 'Claude Opus 4.6', description: '', provider: 'anthropic' },
  { value: 'claude-sonnet-4-5-20250514', displayName: 'Claude Sonnet 4.5', description: '', provider: 'anthropic' },
  { value: 'claude-haiku-4-5-20251001', displayName: 'Claude Haiku 4.5', description: '', provider: 'anthropic' },
  { value: 'claude-3-7-sonnet-20250219', displayName: 'Claude 3.7 Sonnet', description: '', provider: 'anthropic' },
  { value: 'claude-3-5-sonnet-20241022', displayName: 'Claude 3.5 Sonnet', description: '', provider: 'anthropic' },
  { value: 'claude-3-5-haiku-20241022', displayName: 'Claude 3.5 Haiku', description: '', provider: 'anthropic' },
]

/** Connect to Claude Code via Agent SDK and fetch real supported models */
async function connectClaudeCode(): Promise<ConnectResult> {
  serverLog.info('[connect-agent] connecting to Claude Code...')
  const claudePath = resolveClaudeCli()
  serverLog.info(`[connect-agent] resolved claude path: ${claudePath ?? 'NOT FOUND'}`)
  if (!claudePath) {
    return { connected: false, models: [], notInstalled: true, error: 'Claude Code CLI not found' }
  }

  try {
    const { query } = await import('@anthropic-ai/claude-agent-sdk')

    const env = buildClaudeAgentEnv()
    const debugFile = getClaudeAgentDebugFilePath()
    serverLog.info(`[connect-agent] claude env keys: ${Object.keys(env).join(', ')}`)
    serverLog.info(`[connect-agent] claude debugFile: ${debugFile ?? 'none'}`)

    const spawnProcess = buildSpawnClaudeCodeProcess()

    const q = query({
      prompt: '',
      options: {
        maxTurns: 1,
        tools: [],
        permissionMode: 'plan',
        persistSession: false,
        env,
        ...(debugFile ? { debugFile } : {}),
        ...(claudePath ? { pathToClaudeCodeExecutable: claudePath } : {}),
        ...(spawnProcess ? { spawnClaudeCodeProcess: spawnProcess } : {}),
      },
    })

    serverLog.info('[connect-agent] querying supportedModels...')
    const raw = await q.supportedModels()

    // Fetch account info (email, org, subscription type)
    let account: { email?: string; organization?: string; subscriptionType?: string; apiKeySource?: string } | null = null
    try {
      account = await q.accountInfo()
      serverLog.info(`[connect-agent] claude account: email=${account?.email ?? 'n/a'}, type=${account?.subscriptionType ?? 'n/a'}, source=${account?.apiKeySource ?? 'n/a'}`)
    } catch {
      serverLog.info('[connect-agent] accountInfo() not available')
    }
    q.close()

    const models: GroupedModel[] = raw.map((m) => ({
      value: m.value,
      displayName: m.displayName,
      description: m.description,
      provider: 'anthropic' as const,
    }))

    serverLog.info(`[connect-agent] claude connected, ${models.length} models found`)
    const claudeInfo = buildClaudeConnectionInfo(env, account)
    return { connected: true, models, ...claudeInfo }
  } catch (error) {
    const msg = error instanceof Error ? error.message : 'Failed to connect'
    serverLog.error(`[connect-agent] claude connection error: ${msg}`)
    // Third-party API proxies often don't support the supportedModels() call,
    // causing "query closed before response". Fall back to a default model list
    // so users can still connect and choose a model.
    if (/closed before|closed early|query closed/i.test(msg)) {
      serverLog.info('[connect-agent] using fallback model list (proxy detected)')
      const fallbackEnv = buildClaudeAgentEnv()
      const claudeInfo = buildClaudeConnectionInfo(fallbackEnv, null)

      // Read debug log for diagnostic warning — the process may have written
      // useful info (e.g. TLS errors, auth failures) before exiting
      let warning: string | undefined
      const debugPath = getClaudeAgentDebugFilePath()
      if (debugPath) {
        try {
          const raw = readFileSync(debugPath, 'utf-8')
          const lines = raw.split('\n').filter((l) => l.trim().length > 0)
          const tail = lines.slice(-10).join('\n')
          if (tail) {
            // Surface specific issues as warnings
            if (/certificate|CERT_|ssl|tls/i.test(tail)) {
              warning = 'TLS/SSL error detected. If using a proxy, add "NODE_TLS_REJECT_UNAUTHORIZED": "0" to ~/.claude/settings.json env.'
            } else if (/EPERM|operation not permitted/i.test(tail)) {
              warning = 'Permission error writing config. Try: echo {} > %USERPROFILE%\\.claude.json'
            } else if (/stderr exit=/i.test(tail)) {
              // Show captured stderr
              const stderrMatch = tail.match(/\[stderr exit=\d+\]\s*(.+)/s)
              if (stderrMatch) {
                warning = `Claude Code stderr: ${stderrMatch[1].slice(0, 300)}`
              }
            }
          }
        } catch { /* debug file not available */ }
      }

      return { connected: true, models: FALLBACK_CLAUDE_MODELS, ...claudeInfo, ...(warning ? { warning } : {}) }
    }
    return { connected: false, models: [], error: friendlyClaudeError(msg) }
  }
}

/** Resolve config file path (cross-platform) */
function configPath(unixPath: string, winPath: string): string {
  return process.platform === 'win32' ? winPath : unixPath
}

/** Build Claude connection info from env + SDK account info */
function buildClaudeConnectionInfo(
  env: Record<string, string | undefined>,
  account: { email?: string; organization?: string; subscriptionType?: string; apiKeySource?: string } | null,
): { connectionInfo: string; hintPath?: string } {
  const hp = configPath('~/.claude/settings.json', '%USERPROFILE%\\.claude\\settings.json')
  const apiKey = env.ANTHROPIC_API_KEY
  const baseUrl = env.ANTHROPIC_BASE_URL

  if (account?.email) {
    const sub = account.subscriptionType ?? 'subscription'
    return { connectionInfo: `Connected via ${sub} (${account.email})`, hintPath: hp }
  }
  if (apiKey && baseUrl) {
    return { connectionInfo: 'Connected via API key (custom endpoint)', hintPath: hp }
  }
  if (apiKey) {
    const masked = apiKey.length > 12 ? `${apiKey.slice(0, 8)}...` : '***'
    return { connectionInfo: `Connected via API key (${masked})`, hintPath: hp }
  }
  return { connectionInfo: 'Connected via subscription', hintPath: hp }
}

/** Decode a JWT payload (no verification — just base64url decode the middle part) */
function decodeJwtPayload(token: string): Record<string, unknown> | null {
  try {
    const parts = token.split('.')
    if (parts.length !== 3) return null
    // base64url → base64 → Buffer → JSON
    const b64 = parts[1].replace(/-/g, '+').replace(/_/g, '/')
    const padded = b64 + '='.repeat((4 - (b64.length % 4)) % 4)
    return JSON.parse(Buffer.from(padded, 'base64').toString('utf-8'))
  } catch {
    return null
  }
}

/** Build Codex CLI connection info by reading ~/.codex/auth.json + JWT tokens */
async function buildCodexConnectionInfo(): Promise<{ connectionInfo: string; hintPath?: string }> {
  const { readFile } = await import('node:fs/promises')
  const { homedir } = await import('node:os')
  const { join } = await import('node:path')
  const hp = configPath('~/.codex/config.json', '%USERPROFILE%\\.codex\\config.json')

  if (process.env.OPENAI_API_KEY) {
    const key = process.env.OPENAI_API_KEY
    const masked = key.length > 12 ? `${key.slice(0, 8)}...` : '***'
    return { connectionInfo: `Connected via API key (${masked})`, hintPath: hp }
  }

  try {
    const codexHome = process.env.CODEX_HOME || join(homedir(), '.codex')
    const authPath = join(codexHome, 'auth.json')
    const raw = await readFile(authPath, 'utf-8')
    const auth = JSON.parse(raw) as { auth_mode?: string; tokens?: { id_token?: string } }

    const idToken = auth.tokens?.id_token
    if (idToken) {
      const payload = decodeJwtPayload(idToken)
      if (payload) {
        const email = payload.email as string | undefined
        const authClaims = payload['https://api.openai.com/auth'] as Record<string, unknown> | undefined
        const plan = authClaims?.chatgpt_plan_type as string | undefined
        serverLog.info(`[connect-agent] codex JWT: email=${email ?? 'n/a'}, plan=${plan ?? 'n/a'}`)
        if (email) {
          const label = plan ?? auth.auth_mode ?? 'subscription'
          return { connectionInfo: `Connected via ${label} (${email})`, hintPath: hp }
        }
      }
    }
    if (auth.auth_mode) {
      return { connectionInfo: `Connected via ${auth.auth_mode}`, hintPath: hp }
    }
  } catch { /* auth.json not found */ }

  return { connectionInfo: 'Connected via Codex CLI', hintPath: hp }
}

/** Map raw Agent SDK errors to user-friendly messages */
function friendlyClaudeError(raw: string): string {
  if (/process exited with code 1|invalid model|unknown model|model.*not/i.test(raw)) {
    return 'Claude Code exited with code 1. Run "claude login" to authenticate, or set ANTHROPIC_API_KEY in ~/.claude/settings.json.'
  }
  if (/exited with code/i.test(raw)) {
    return 'Unable to connect. Claude Code process exited unexpectedly.'
  }
  if (/not found|ENOENT/i.test(raw)) {
    return 'Claude Code CLI not found. Please install it first.'
  }
  if (/timed?\s*out/i.test(raw)) {
    return 'Connection timed out. Please try again.'
  }
  return raw
}

/**
 * Fallback: parse model IDs from Codex's bundled latest-model.md when
 * models_cache.json is missing (e.g. fresh Windows install).
 * Only includes text/reasoning models (skips image, audio, video, embedding, moderation).
 */
async function parseCodexLatestModelMd(codexHome: string): Promise<GroupedModel[]> {
  const { readFile } = await import('node:fs/promises')
  const { join } = await import('node:path')
  const mdPath = join(codexHome, 'skills', '.system', 'openai-docs', 'references', 'latest-model.md')
  try {
    const content = await readFile(mdPath, 'utf-8')
    const models: GroupedModel[] = []
    // Match markdown table rows: | `model-id` | description |
    const rowRe = /^\|\s*`([^`]+)`\s*\|\s*(.+?)\s*\|/gm
    const skipRe = /image|audio|tts|transcribe|realtime|sora|video|embedding|moderation/i
    let match: RegExpExecArray | null
    const seen = new Set<string>()
    while ((match = rowRe.exec(content)) !== null) {
      const slug = match[1]
      const desc = match[2].trim()
      if (skipRe.test(slug) || skipRe.test(desc) || seen.has(slug)) continue
      seen.add(slug)
      models.push({
        value: slug,
        displayName: slug,
        description: desc,
        provider: 'openai' as const,
      })
    }
    return models
  } catch {
    return []
  }
}

/** Connect to Codex CLI and fetch its supported models from the local cache */
async function connectCodexCli(): Promise<ConnectResult> {
  serverLog.info('[connect-agent] connecting to Codex CLI...')
  try {
    const { execSync } = await import('node:child_process')
    const { readFile } = await import('node:fs/promises')
    const { homedir } = await import('node:os')
    const { join } = await import('node:path')
    const isWin = process.platform === 'win32'

    // Check if codex binary exists — PATH, npm prefix, then common locations
    let which = ''

    // 1. PATH lookup
    try {
      const whichCmd = isWin ? 'where codex 2>nul' : 'which codex 2>/dev/null || echo ""'
      serverLog.info(`[connect-agent] codex PATH lookup: ${whichCmd}`)
      const result = execSync(whichCmd, {
        encoding: 'utf-8',
        timeout: 5000,
      }).trim().split(/\r?\n/)[0]?.trim() ?? ''
      if (result && existsSync(result)) which = resolveWinExtension(result)
      serverLog.info(`[connect-agent] codex PATH result: "${result}" resolved="${which}" (exists=${result ? existsSync(result) : false})`)
    } catch (err) {
      serverLog.info(`[connect-agent] codex PATH lookup failed: ${err instanceof Error ? err.message : err}`)
    }

    // 2. npm prefix -g (Windows: npm global creates .cmd or .ps1 wrappers)
    if (!which && isWin) {
      try {
        serverLog.info('[connect-agent] codex: trying npm.cmd prefix -g')
        const prefix = execSync('npm.cmd prefix -g', {
          encoding: 'utf-8',
          timeout: 5000,
        }).trim()
        serverLog.info(`[connect-agent] codex npm global prefix: "${prefix}"`)
        if (prefix) {
          for (const bin of winNpmCandidates(prefix, 'codex')) {
            serverLog.info(`[connect-agent] codex npm global bin: "${bin}" (exists=${existsSync(bin)})`)
            if (existsSync(bin)) { which = bin; break }
          }
        }
      } catch (err) {
        serverLog.info(`[connect-agent] codex npm prefix -g failed: ${err instanceof Error ? err.message : err}`)
      }
    }

    // 3. Common install locations
    if (!which && isWin) {
      const candidates = [
        ...winNpmCandidates(join(process.env.APPDATA || '', 'npm'), 'codex'),
        ...winNpmCandidates(join(process.env.NVM_SYMLINK || ''), 'codex'),
        ...winNpmCandidates(join(process.env.FNM_MULTISHELL_PATH || ''), 'codex'),
      ]
      for (const c of candidates) {
        const exists = c ? existsSync(c) : false
        serverLog.info(`[connect-agent] codex candidate: "${c}" (exists=${exists})`)
        if (c && exists) { which = c; break }
      }
    }

    if (!which) {
      serverLog.warn('[connect-agent] codex not found')
      return { connected: false, models: [], notInstalled: true, error: 'Codex CLI not found' }
    }
    serverLog.info(`[connect-agent] codex resolved: "${which}"`)


    // Verify codex is responsive — always use the resolved path
    const versionCmd = buildExecCmd(which, '--version') + ' 2>&1'
    try {
      const ver = execSync(versionCmd, { encoding: 'utf-8', timeout: 5000 }).trim()
      serverLog.info(`[connect-agent] codex version: ${ver}`)
    } catch (err) {
      serverLog.error(`[connect-agent] codex --version failed: ${err instanceof Error ? err.message : err}`)
      return { connected: false, models: [], error: 'Codex CLI not responding' }
    }

    // Read models from Codex CLI's local models cache (best-effort)
    let models: GroupedModel[] = []
    const codexHome = process.env.CODEX_HOME || join(homedir(), '.codex')
    const cachePath = join(codexHome, 'models_cache.json')
    try {
      const raw = await readFile(cachePath, 'utf-8')
      const cache = JSON.parse(raw) as {
        models?: Array<{
          slug: string
          display_name: string
          description: string
          visibility: string
          priority: number
        }>
      }
      if (cache.models && Array.isArray(cache.models)) {
        models = cache.models
          .filter((m) => m.visibility === 'list')
          .sort((a, b) => (a.priority ?? 999) - (b.priority ?? 999))
          .map((m) => ({
            value: m.slug,
            displayName: m.display_name,
            description: m.description ?? '',
            provider: 'openai' as const,
          }))
      }
    } catch {
      serverLog.info(`[connect-agent] codex models cache not available`)
    }

    // Fallback: parse models from Codex's bundled latest-model.md reference
    if (models.length === 0) {
      models = await parseCodexLatestModelMd(codexHome)
      if (models.length > 0) {
        serverLog.info(`[connect-agent] codex models loaded from latest-model.md: ${models.length}`)
      }
    }

    serverLog.info(`[connect-agent] codex connected, ${models.length} models found`)
    const codexInfo = await buildCodexConnectionInfo()
    const warning = models.length === 0 ? 'No models found. Try running codex once to populate the model cache.' : undefined
    return { connected: true, models, warning, ...codexInfo }
  } catch (error) {
    const msg = error instanceof Error ? error.message : 'Failed to connect'
    serverLog.error(`[connect-agent] codex connection error: ${msg}`)
    return { connected: false, models: [], error: msg }
  }
}

/** Resolve the opencode binary path, checking PATH then common install locations. */
async function resolveOpencodeBinary(): Promise<string | undefined> {
  const { execSync } = await import('node:child_process')
  const { existsSync } = await import('node:fs')
  const { homedir } = await import('node:os')
  const { join } = await import('node:path')
  const isWin = process.platform === 'win32'

  serverLog.info(`[resolve-opencode] platform=${process.platform}, isWindows=${isWin}`)

  // 1. Try PATH lookup
  try {
    const cmd = isWin ? 'where opencode 2>nul' : 'which opencode 2>/dev/null'
    serverLog.info(`[resolve-opencode] PATH lookup: ${cmd}`)
    const result = execSync(cmd, { encoding: 'utf-8', timeout: 5000 }).trim().split(/\r?\n/)[0]?.trim()
    serverLog.info(`[resolve-opencode] PATH result: "${result}" (exists=${result ? existsSync(result) : false})`)
    if (result && existsSync(result)) return resolveWinExtension(result)
  } catch (err) {
    serverLog.info(`[resolve-opencode] PATH lookup failed: ${err instanceof Error ? err.message : err}`)
  }

  // 2. Try `npm prefix -g` to find actual npm global bin directory
  //    On Windows, must use `npm.cmd` since Electron spawns cmd.exe
  try {
    const npmCmd = isWin ? 'npm.cmd prefix -g' : 'npm prefix -g'
    serverLog.info(`[resolve-opencode] npm prefix lookup: ${npmCmd}`)
    const prefix = execSync(npmCmd, { encoding: 'utf-8', timeout: 5000 }).trim()
    serverLog.info(`[resolve-opencode] npm global prefix: "${prefix}"`)
    if (prefix) {
      if (isWin) {
        for (const bin of winNpmCandidates(prefix, 'opencode')) {
          serverLog.info(`[resolve-opencode] npm global bin: "${bin}" (exists=${existsSync(bin)})`)
          if (existsSync(bin)) return bin
        }
      } else {
        const bin = join(prefix, 'bin', 'opencode')
        serverLog.info(`[resolve-opencode] npm global bin: "${bin}" (exists=${existsSync(bin)})`)
        if (existsSync(bin)) return bin
      }
    }
  } catch (err) {
    serverLog.info(`[resolve-opencode] npm prefix -g failed: ${err instanceof Error ? err.message : err}`)
  }

  // 3. Common install locations
  //    npm -g → %APPDATA%\npm (Windows), /usr/local (macOS/Linux)
  //    curl installer → ~/.opencode/bin (macOS/Linux)
  //    Homebrew → /usr/local/bin or /opt/homebrew/bin (macOS)
  const home = homedir()
  const candidates = isWin
    ? [
        // npm global (.cmd + .ps1)
        ...winNpmCandidates(join(process.env.APPDATA || '', 'npm'), 'opencode'),
        ...winNpmCandidates(join(process.env.ProgramFiles || '', 'nodejs'), 'opencode'),
        // nvm-windows / fnm
        ...winNpmCandidates(join(process.env.NVM_SYMLINK || ''), 'opencode'),
        ...winNpmCandidates(join(process.env.FNM_MULTISHELL_PATH || ''), 'opencode'),
        // Scoop
        join(home, 'scoop', 'shims', 'opencode.exe'),
        join(process.env.LOCALAPPDATA || '', 'Programs', 'opencode', 'opencode.exe'),
      ]
    : [
        // curl installer (https://opencode.ai/install)
        join(home, '.opencode', 'bin', 'opencode'),
        // npm global
        join(home, '.npm-global', 'bin', 'opencode'),
        '/usr/local/bin/opencode',
        // Homebrew
        '/opt/homebrew/bin/opencode',
        join(home, '.local', 'bin', 'opencode'),
      ]
  for (const c of candidates) {
    const exists = c ? existsSync(c) : false
    serverLog.info(`[resolve-opencode] candidate: "${c}" (exists=${exists})`)
    if (c && exists) return c
  }

  serverLog.info('[resolve-opencode] no opencode binary found')
  return undefined
}

/** Connect to OpenCode and fetch its configured providers/models. */
async function connectOpenCode(): Promise<ConnectResult> {
  serverLog.info('[connect-agent] connecting to OpenCode...')
  try {
    const binaryPath = await resolveOpencodeBinary()
    serverLog.info(`[connect-agent] resolved opencode path: ${binaryPath ?? 'NOT FOUND'}`)
    if (!binaryPath) {
      return { connected: false, models: [], notInstalled: true, error: 'OpenCode CLI not found' }
    }

    const { getOpencodeClient, releaseOpencodeServer } = await import('../../utils/opencode-client')
    serverLog.info('[connect-agent] creating opencode client...')
    const { client, server } = await getOpencodeClient()

    serverLog.info('[connect-agent] fetching opencode providers...')
    const { data, error } = await client.config.providers()
    releaseOpencodeServer(server)

    if (error) {
      serverLog.error(`[connect-agent] opencode providers error: ${JSON.stringify(error)}`)
      return { connected: false, models: [], error: 'Failed to fetch providers from OpenCode server.' }
    }

    const models: GroupedModel[] = []
    for (const provider of data?.providers ?? []) {
      if (!provider.models) continue
      for (const [, model] of Object.entries(provider.models)) {
        models.push({
          value: `${provider.id}/${model.id}`,
          displayName: model.name || model.id,
          description: `via ${provider.name || provider.id}`,
          provider: 'opencode' as const,
        })
      }
    }

    if (models.length === 0) {
      serverLog.info('[connect-agent] opencode: no models found')
      return { connected: false, models: [], error: 'No models configured in OpenCode. Run "opencode" to set up providers.' }
    }

    const providerNames = (data?.providers ?? []).map((p) => p.name || p.id).filter(Boolean)
    const providerSummary = providerNames.length > 0
      ? `Connected (${providerNames.slice(0, 3).join(', ')}${providerNames.length > 3 ? ` +${providerNames.length - 3}` : ''})`
      : 'Connected via OpenCode server'
    serverLog.info(`[connect-agent] opencode connected, ${models.length} models found`)
    return {
      connected: true, models,
      connectionInfo: providerSummary,
      hintPath: configPath('~/.opencode/config.json', '%USERPROFILE%\\.opencode\\config.json'),
    }
  } catch (error) {
    const raw = error instanceof Error ? error.message : 'Failed to connect'
    serverLog.error(`[connect-agent] opencode connection error: ${raw}`)
    return { connected: false, models: [], error: friendlyOpenCodeError(raw) }
  }
}

/** Connect to GitHub Copilot CLI via @github/copilot-sdk and fetch available models. */
async function connectCopilot(): Promise<ConnectResult> {
  serverLog.info('[connect-agent] connecting to Copilot...')
  // Use standalone copilot binary to avoid Bun's node:sqlite issue
  const { resolveCopilotCli } = await import('../../utils/copilot-client')
  const cliPath = resolveCopilotCli()
  serverLog.info(`[connect-agent] resolved copilot path: ${cliPath ?? 'NOT FOUND'}`)
  if (!cliPath) {
    return { connected: false, models: [], notInstalled: true, error: 'GitHub Copilot CLI not found' }
  }

  try {
    const { CopilotClient } = await import('@github/copilot-sdk')
    const client = new CopilotClient({ autoStart: true, cliPath })

    serverLog.info('[connect-agent] starting copilot client...')
    await client.start()

    let models: GroupedModel[] = []
    try {
      serverLog.info('[connect-agent] listing copilot models...')
      const modelList = await client.listModels()
      models = modelList
        .filter((m) => !m.policy || m.policy.state === 'enabled')
        .map((m) => ({
          value: m.id,
          displayName: m.name,
          description: m.capabilities?.supports?.vision ? 'vision' : '',
          provider: 'copilot' as const,
        }))
    } catch (listErr) {
      const msg = listErr instanceof Error ? listErr.message : 'Failed to list models'
      serverLog.error(`[connect-agent] copilot listModels error: ${msg}`)
      await client.stop().catch(() => {})
      return { connected: false, models: [], error: friendlyCopilotError(msg) }
    }

    // Try to get auth status for user info
    const copilotHintPath = configPath('~/.config/github-copilot/config.json', '%USERPROFILE%\\.config\\github-copilot\\config.json')
    let copilotInfo: { connectionInfo: string; hintPath?: string } = { connectionInfo: 'Connected via GitHub', hintPath: copilotHintPath }
    try {
      const authStatus = await client.getAuthStatus()
      serverLog.info(`[connect-agent] copilot auth: ${JSON.stringify(authStatus)}`)
      if (authStatus?.login) {
        const method = authStatus.authType ? ` (${authStatus.authType})` : ''
        copilotInfo = { connectionInfo: `Connected as @${authStatus.login}${method}`, hintPath: copilotHintPath }
      } else if (authStatus?.statusMessage) {
        copilotInfo = { connectionInfo: authStatus.statusMessage, hintPath: copilotHintPath }
      }
    } catch (authErr) {
      serverLog.warn(`[connect-agent] copilot getAuthStatus failed: ${authErr instanceof Error ? authErr.message : authErr}`)
    }

    await client.stop()

    if (models.length === 0) {
      serverLog.info('[connect-agent] copilot: no models found')
      return { connected: false, models: [], error: 'No models found. Run "copilot login" to authenticate first.' }
    }

    serverLog.info(`[connect-agent] copilot connected, ${models.length} models found`)
    return { connected: true, models, ...copilotInfo }
  } catch (error) {
    const raw = error instanceof Error ? error.message : 'Failed to connect'
    serverLog.error(`[connect-agent] copilot connection error: ${raw}`)
    return { connected: false, models: [], error: friendlyCopilotError(raw) }
  }
}

/** Map Copilot SDK errors to user-friendly messages */
function friendlyCopilotError(raw: string): string {
  if (/not found|ENOENT/i.test(raw)) {
    return 'GitHub Copilot CLI not found. Install it from https://docs.github.com/copilot/how-tos/copilot-cli'
  }
  if (/not authenticated|authenticate first|auth|unauthenticated|login/i.test(raw)) {
    return 'Not authenticated. Run "copilot login" in your terminal first.'
  }
  if (/timed?\s*out/i.test(raw)) {
    return 'Connection timed out. Please try again.'
  }
  return raw
}

/** Map OpenCode connection errors to user-friendly messages */
function friendlyOpenCodeError(raw: string): string {
  if (/ECONNREFUSED/i.test(raw)) {
    return 'OpenCode server not running. Start it with "opencode" in your terminal first.'
  }
  if (/not found|ENOENT/i.test(raw)) {
    return 'OpenCode CLI not found. Please install it first.'
  }
  if (/timed?\s*out/i.test(raw)) {
    return 'Connection timed out. Please try again.'
  }
  return raw
}

/** Fallback model list when dynamic fetch fails */
const FALLBACK_GEMINI_MODELS: GroupedModel[] = [
  { value: 'gemini-3-pro-preview', displayName: 'Gemini 3 Pro', description: 'Most capable', provider: 'gemini' },
  { value: 'gemini-3-flash-preview', displayName: 'Gemini 3 Flash', description: 'Fast + capable', provider: 'gemini' },
  { value: 'gemini-2.5-pro', displayName: 'Gemini 2.5 Pro', description: 'Thinking model', provider: 'gemini' },
  { value: 'gemini-2.5-flash', displayName: 'Gemini 2.5 Flash', description: 'Fast + thinking', provider: 'gemini' },
  { value: 'gemini-2.0-flash', displayName: 'Gemini 2.0 Flash', description: 'Fast model', provider: 'gemini' },
]

/** Fetch available models from Gemini API using local auth credentials */
async function fetchGeminiModels(): Promise<GroupedModel[]> {
  const { readFile } = await import('node:fs/promises')
  const { homedir } = await import('node:os')
  const { join } = await import('node:path')

  // Build auth header — try API key first, then OAuth token
  let authUrl: (base: string) => string
  let headers: Record<string, string> = {}

  const envKey = process.env.GEMINI_API_KEY || process.env.GOOGLE_API_KEY
  if (envKey) {
    authUrl = (base) => `${base}?key=${envKey}`
  } else {
    // Read OAuth token
    const oauthPath = join(homedir(), '.gemini', 'oauth_creds.json')
    const raw = await readFile(oauthPath, 'utf-8')
    const creds = JSON.parse(raw) as { access_token?: string; expiry_date?: number }
    if (!creds.access_token) throw new Error('No access token')
    if (creds.expiry_date && Date.now() > creds.expiry_date - 60_000) throw new Error('Token expired')
    authUrl = (base) => base
    headers = { Authorization: `Bearer ${creds.access_token}` }
  }

  const res = await fetch(authUrl('https://generativelanguage.googleapis.com/v1beta/models'), { headers })
  if (!res.ok) throw new Error(`API ${res.status}`)

  const data = await res.json() as {
    models?: Array<{
      name?: string
      displayName?: string
      description?: string
      supportedGenerationMethods?: string[]
    }>
  }

  const models: GroupedModel[] = []
  const seen = new Set<string>()
  for (const m of data.models ?? []) {
    // Only include models that support generateContent (text generation)
    if (!m.supportedGenerationMethods?.includes('generateContent')) continue
    const id = m.name?.replace('models/', '') ?? ''
    if (!id || seen.has(id)) continue
    // Skip embedding, AQA, and legacy models
    if (/embed|aqa|^chat-bison|^text-bison|^gemini-1\.0/i.test(id)) continue
    seen.add(id)
    models.push({
      value: id,
      displayName: m.displayName ?? id,
      description: m.description?.slice(0, 60) ?? '',
      provider: 'gemini' as const,
    })
  }

  // Sort: gemini-3 first, then 2.5, then others
  models.sort((a, b) => {
    const order = (v: string) => {
      if (v.includes('gemini-3')) return 0
      if (v.includes('gemini-2.5-pro')) return 1
      if (v.includes('gemini-2.5-flash')) return 2
      if (v.includes('gemini-2.0')) return 3
      return 4
    }
    return order(a.value) - order(b.value)
  })

  return models
}

/** Connect to Gemini CLI and return available models. */
async function connectGeminiCli(): Promise<ConnectResult> {
  serverLog.info('[connect-agent] connecting to Gemini CLI...')
  try {
    const { resolveGeminiCli } = await import('../../utils/resolve-gemini-cli')
    const binPath = resolveGeminiCli()
    serverLog.info(`[connect-agent] resolved gemini path: ${binPath ?? 'NOT FOUND'}`)
    if (!binPath) {
      return { connected: false, models: [], notInstalled: true, error: 'Gemini CLI not found' }
    }

    // Verify binary responds
    const { execSync } = await import('node:child_process')
    const versionCmd = buildExecCmd(binPath, '--version')
    try {
      const ver = execSync(`${versionCmd} 2>&1`, { encoding: 'utf-8', timeout: 10000 }).trim()
      serverLog.info(`[connect-agent] gemini version: ${ver}`)
    } catch (err) {
      serverLog.error(`[connect-agent] gemini --version failed: ${err instanceof Error ? err.message : err}`)
      return { connected: false, models: [], error: 'Gemini CLI not responding' }
    }

    // Dynamically fetch models, fallback to hardcoded list
    let models: GroupedModel[]
    try {
      models = await fetchGeminiModels()
      serverLog.info(`[connect-agent] gemini: fetched ${models.length} models from API`)
    } catch (err) {
      serverLog.info(`[connect-agent] gemini: model fetch failed (${err instanceof Error ? err.message : err}), using fallback`)
      models = FALLBACK_GEMINI_MODELS
    }

    const geminiInfo = await buildGeminiConnectionInfo()
    const warning = models.length === 0 ? 'No models found. Try running "gemini" once to authenticate.' : undefined
    if (models.length === 0) models = FALLBACK_GEMINI_MODELS
    serverLog.info(`[connect-agent] gemini connected, ${models.length} models`)
    return { connected: true, models, warning, ...geminiInfo }
  } catch (error) {
    const raw = error instanceof Error ? error.message : 'Failed to connect'
    serverLog.error(`[connect-agent] gemini connection error: ${raw}`)
    return { connected: false, models: [], error: friendlyGeminiError(raw) }
  }
}

/** Build Gemini CLI connection info from local config files */
async function buildGeminiConnectionInfo(): Promise<{ connectionInfo: string; hintPath?: string }> {
  const { readFile } = await import('node:fs/promises')
  const { homedir } = await import('node:os')
  const { join } = await import('node:path')
  const hp = configPath('~/.gemini/settings.json', '%USERPROFILE%\\.gemini\\settings.json')

  // Check env for API key
  const envKey = process.env.GEMINI_API_KEY || process.env.GOOGLE_API_KEY
  if (envKey) {
    const masked = envKey.length > 12 ? `${envKey.slice(0, 8)}...` : '***'
    return { connectionInfo: `Connected via API key (${masked})`, hintPath: hp }
  }

  // Check OAuth creds (Gemini CLI login)
  try {
    const oauthPath = join(homedir(), '.gemini', 'oauth_creds.json')
    await readFile(oauthPath, 'utf-8') // Check existence

    // Try to get account email
    try {
      const accountsPath = join(homedir(), '.gemini', 'google_accounts.json')
      const accountsRaw = await readFile(accountsPath, 'utf-8')
      const accounts = JSON.parse(accountsRaw) as { active?: string }
      if (accounts.active) {
        return { connectionInfo: `Connected via Google (${accounts.active})`, hintPath: hp }
      }
    } catch { /* no accounts file */ }

    return { connectionInfo: 'Connected via Google OAuth', hintPath: hp }
  } catch { /* no OAuth creds */ }

  return { connectionInfo: 'Connected via Gemini CLI', hintPath: hp }
}

/** Map Gemini CLI errors to user-friendly messages */
function friendlyGeminiError(raw: string): string {
  if (/not found|ENOENT/i.test(raw)) {
    return 'Gemini CLI not found. Install it with: npm install -g @anthropic-ai/gemini-cli'
  }
  if (/not authenticated|authenticate|auth|login/i.test(raw)) {
    return 'Not authenticated. Run "gemini" in your terminal first to set up authentication.'
  }
  if (/timed?\s*out/i.test(raw)) {
    return 'Connection timed out. Please try again.'
  }
  return raw
}
