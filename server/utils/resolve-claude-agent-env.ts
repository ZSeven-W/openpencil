import { mkdirSync, readFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { join } from 'node:path'

type EnvLike = Record<string, string | undefined>

interface ClaudeSettings {
  env?: Record<string, unknown>
}

function normalizeEnvValue(value: unknown): string | undefined {
  if (value == null) return undefined
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value)
  }
  return undefined
}

function readClaudeSettingsEnv(): EnvLike {
  try {
    const path = join(homedir(), '.claude', 'settings.json')
    const raw = readFileSync(path, 'utf-8')
    const parsed = JSON.parse(raw) as ClaudeSettings
    if (!parsed.env || typeof parsed.env !== 'object') return {}

    const env: EnvLike = {}
    for (const [key, value] of Object.entries(parsed.env)) {
      const normalized = normalizeEnvValue(value)
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
 * Build env passed to Claude Agent SDK.
 * Priority: current process env > ~/.claude/settings.json env.
 */
export function buildClaudeAgentEnv(): EnvLike {
  const merged: EnvLike = {
    ...readClaudeSettingsEnv(),
    ...(process.env as EnvLike),
  }

  // Compatibility: some Claude-compatible gateways expose token as ANTHROPIC_AUTH_TOKEN.
  // Claude Code primarily understands ANTHROPIC_API_KEY / ANTHROPIC_CUSTOM_HEADERS.
  const authToken = merged.ANTHROPIC_AUTH_TOKEN
  if (authToken && !merged.ANTHROPIC_API_KEY) {
    merged.ANTHROPIC_API_KEY = authToken
  }
  if (authToken && !merged.ANTHROPIC_CUSTOM_HEADERS) {
    merged.ANTHROPIC_CUSTOM_HEADERS = JSON.stringify({
      Authorization: `Bearer ${authToken}`,
    })
  }

  // Running inside Claude terminal can break nested Claude invocations.
  delete merged.CLAUDECODE
  return merged
}

/**
 * Force Claude CLI debug output into a writable temp location.
 * This avoids crashes in restricted environments where ~/.claude/debug is not writable.
 */
export function getClaudeAgentDebugFilePath(): string | undefined {
  try {
    const dir = join('/tmp', 'openpencil-claude-debug')
    mkdirSync(dir, { recursive: true })
    return join(dir, 'claude-agent.log')
  } catch {
    return undefined
  }
}
