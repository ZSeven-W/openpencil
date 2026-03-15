import { defineEventHandler, readBody, setResponseHeaders } from 'h3'
import type { GroupedModel } from '../../../src/types/agent-settings'
import { resolveClaudeCli } from '../../utils/resolve-claude-cli'
import {
  buildClaudeAgentEnv,
  getClaudeAgentDebugFilePath,
} from '../../utils/resolve-claude-agent-env'

interface ConnectBody {
  agent: 'claude-code' | 'codex-cli' | 'opencode' | 'copilot'
}

interface ConnectResult {
  connected: boolean
  models: GroupedModel[]
  error?: string
  notInstalled?: boolean
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
  const claudePath = resolveClaudeCli()
  if (!claudePath) {
    return { connected: false, models: [], notInstalled: true, error: 'Claude Code CLI not found' }
  }

  try {
    const { query } = await import('@anthropic-ai/claude-agent-sdk')

    const env = buildClaudeAgentEnv()
    const debugFile = getClaudeAgentDebugFilePath()

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
      },
    })

    const raw = await q.supportedModels()
    q.close()

    const models: GroupedModel[] = raw.map((m) => ({
      value: m.value,
      displayName: m.displayName,
      description: m.description,
      provider: 'anthropic' as const,
    }))

    return { connected: true, models }
  } catch (error) {
    const msg = error instanceof Error ? error.message : 'Failed to connect'
    // Third-party API proxies often don't support the supportedModels() call,
    // causing "query closed before response". Fall back to a default model list
    // so users can still connect and choose a model.
    if (/closed before|closed early|query closed/i.test(msg)) {
      return { connected: true, models: FALLBACK_CLAUDE_MODELS }
    }
    return { connected: false, models: [], error: friendlyClaudeError(msg) }
  }
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

/** Connect to Codex CLI and fetch its supported models from the local cache */
async function connectCodexCli(): Promise<ConnectResult> {
  try {
    const { execSync } = await import('node:child_process')
    const { readFile } = await import('node:fs/promises')
    const { homedir } = await import('node:os')
    const { join } = await import('node:path')

    // Check if codex binary exists
    const whichCmd = process.platform === 'win32' ? 'where codex 2>nul' : 'which codex 2>/dev/null || echo ""'
    const which = execSync(whichCmd, {
      encoding: 'utf-8',
      timeout: 5000,
    }).trim().split(/\r?\n/)[0]?.trim() ?? ''

    if (!which) {
      return { connected: false, models: [], notInstalled: true, error: 'Codex CLI not found' }
    }

    // Verify codex is responsive
    try {
      execSync('codex --version 2>&1', { encoding: 'utf-8', timeout: 5000 })
    } catch {
      return { connected: false, models: [], error: 'Codex CLI not responding' }
    }

    // Read models from Codex CLI's local models cache
    let models: GroupedModel[] = []
    const cachePath = join(homedir(), '.codex', 'models_cache.json')

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
      // Cache file not found or unreadable
    }

    if (models.length === 0) {
      return { connected: false, models: [], error: 'No models found. Try running codex once to populate the model cache.' }
    }

    return { connected: true, models }
  } catch (error) {
    const msg = error instanceof Error ? error.message : 'Failed to connect'
    return { connected: false, models: [], error: msg }
  }
}

/** Connect to OpenCode and fetch its configured providers/models. */
async function connectOpenCode(): Promise<ConnectResult> {
  try {
    const { execSync } = await import('node:child_process')
    const whichCmd = process.platform === 'win32' ? 'where opencode 2>nul' : 'which opencode 2>/dev/null || echo ""'
    const whichResult = execSync(whichCmd, { encoding: 'utf-8', timeout: 5000 }).trim()
    if (!whichResult) {
      return { connected: false, models: [], notInstalled: true, error: 'OpenCode CLI not found' }
    }

    const { getOpencodeClient, releaseOpencodeServer } = await import('../../utils/opencode-client')
    const { client, server } = await getOpencodeClient()

    const { data, error } = await client.config.providers()
    releaseOpencodeServer(server)

    if (error) {
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
      return { connected: false, models: [], error: 'No models configured in OpenCode. Run "opencode" to set up providers.' }
    }

    return { connected: true, models }
  } catch (error) {
    const raw = error instanceof Error ? error.message : 'Failed to connect'
    return { connected: false, models: [], error: friendlyOpenCodeError(raw) }
  }
}

/** Connect to GitHub Copilot CLI via @github/copilot-sdk and fetch available models. */
async function connectCopilot(): Promise<ConnectResult> {
  // Use standalone copilot binary to avoid Bun's node:sqlite issue
  const { resolveCopilotCli } = await import('../../utils/copilot-client')
  const cliPath = resolveCopilotCli()
  if (!cliPath) {
    return { connected: false, models: [], notInstalled: true, error: 'GitHub Copilot CLI not found' }
  }

  try {
    const { CopilotClient } = await import('@github/copilot-sdk')
    const client = new CopilotClient({ autoStart: true, cliPath })

    await client.start()

    let models: GroupedModel[] = []
    try {
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
      await client.stop().catch(() => {})
      return { connected: false, models: [], error: friendlyCopilotError(msg) }
    }

    await client.stop()

    if (models.length === 0) {
      return { connected: false, models: [], error: 'No models found. Run "copilot login" to authenticate first.' }
    }

    return { connected: true, models }
  } catch (error) {
    const raw = error instanceof Error ? error.message : 'Failed to connect'
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
