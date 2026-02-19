import { defineEventHandler, readBody, setResponseHeaders } from 'h3'
import type { GroupedModel } from '../../../src/types/agent-settings'

interface ConnectBody {
  agent: 'claude-code' | 'codex-cli'
}

interface ConnectResult {
  connected: boolean
  models: GroupedModel[]
  error?: string
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

  return { connected: false, models: [], error: `Unknown agent: ${body.agent}` } satisfies ConnectResult
})

/** Connect to Claude Code via Agent SDK and fetch real supported models */
async function connectClaudeCode(): Promise<ConnectResult> {
  try {
    const { query } = await import('@anthropic-ai/claude-agent-sdk')

    const env = { ...process.env } as Record<string, string | undefined>
    delete env.CLAUDECODE

    const q = query({
      prompt: '',
      options: {
        model: 'claude-sonnet-4-6',
        maxTurns: 1,
        tools: [],
        permissionMode: 'plan',
        persistSession: false,
        env,
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
    const raw = error instanceof Error ? error.message : 'Failed to connect'
    return { connected: false, models: [], error: friendlyClaudeError(raw) }
  }
}

/** Map raw Agent SDK errors to user-friendly messages */
function friendlyClaudeError(raw: string): string {
  if (/exited with code/i.test(raw)) {
    return 'Unable to connect. Please run "claude login" in your terminal first.'
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
    const which = execSync('which codex 2>/dev/null || echo ""', {
      encoding: 'utf-8',
      timeout: 5000,
    }).trim()

    if (!which) {
      return { connected: false, models: [], error: 'codex command not found. Install Codex CLI first.' }
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
