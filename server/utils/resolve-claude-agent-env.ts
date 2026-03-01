import { mkdirSync, readFileSync } from 'node:fs'
import { homedir } from 'node:os'
import { join } from 'node:path'

type EnvLike = Record<string, string | undefined>

interface ClaudeSettings {
  env?: Record<string, unknown>
}

function normalizeEnvValue(value: unknown): string | undefined {
  if (value == null) return undefined
  if (typeof value === 'string') {
    // Filter out empty strings - they cause issues
    if (value.trim() === '') return undefined
    return value
  }
  // Skip non-string values (objects, arrays, numbers, booleans) to prevent
  // invalid header errors like "Invalid header name: '{...}'"
  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value)
  }
  // Skip object values - they would cause header errors
  if (typeof value === 'object') {
    return undefined
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
 * Build env passed to Claude Agent SDK.
 * Priority: current process env > ~/.claude/settings.json env.
 */
export function buildClaudeAgentEnv(): EnvLike {
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
