import { spawn } from 'node:child_process'
import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

type ThinkingMode = 'adaptive' | 'disabled' | 'enabled'
type ThinkingEffort = 'low' | 'medium' | 'high' | 'max'

interface CodexExecOptions {
  model?: string
  systemPrompt?: string
  thinkingMode?: ThinkingMode
  thinkingBudgetTokens?: number
  effort?: ThinkingEffort
  timeoutMs?: number
}

interface CodexCliResult {
  text?: string
  error?: string
}

const DEFAULT_CODEX_TIMEOUT_MS = 15 * 60 * 1000

export async function runCodexExec(
  userPrompt: string,
  options: CodexExecOptions = {},
): Promise<CodexCliResult> {
  const tempDir = await mkdtemp(join(tmpdir(), 'openpencil-codex-'))
  const outputPath = join(tempDir, 'last-message.txt')
  const prompt = buildPrompt(options.systemPrompt, userPrompt)
  const codexEffort = resolveCodexEffort(options.thinkingMode, options.effort)

  const args = [
    'exec',
    '--json',
    '--skip-git-repo-check',
    '--sandbox',
    'read-only',
    '--output-last-message',
    outputPath,
  ]

  if (options.model) {
    args.push('--model', options.model)
  }

  if (codexEffort) {
    args.push('--config', `model_reasoning_effort="${codexEffort}"`)
  }

  args.push(prompt)

  try {
    const runResult = await executeCodexCommand(
      args,
      options.timeoutMs ?? DEFAULT_CODEX_TIMEOUT_MS,
    )
    const finalText = await readFile(outputPath, 'utf-8').catch(() => '')
    const normalizedText = finalText.trim() || runResult.text.trim()

    if (normalizedText) {
      return { text: normalizedText }
    }

    if (runResult.errors.length > 0) {
      return { error: runResult.errors.join('; ') }
    }

    return { error: 'Codex returned no output.' }
  } catch (error) {
    return { error: error instanceof Error ? error.message : 'Codex execution failed' }
  } finally {
    await rm(tempDir, { recursive: true, force: true }).catch(() => {})
  }
}

function buildPrompt(systemPrompt: string | undefined, userPrompt: string): string {
  const userText = userPrompt.trim()
  if (!systemPrompt?.trim()) {
    return userText
  }

  return [
    'SYSTEM INSTRUCTIONS:',
    systemPrompt.trim(),
    '',
    'USER REQUEST:',
    userText,
  ].join('\n')
}

function resolveCodexEffort(
  thinkingMode: ThinkingMode | undefined,
  effort: ThinkingEffort | undefined,
): 'low' | 'medium' | 'high' | undefined {
  if (thinkingMode === 'disabled') {
    return 'low'
  }

  if (effort === 'max') {
    return 'high'
  }

  if (effort === 'low' || effort === 'medium' || effort === 'high') {
    return effort
  }

  if (thinkingMode === 'enabled') {
    return 'medium'
  }

  return undefined
}

async function executeCodexCommand(
  args: string[],
  timeoutMs: number,
): Promise<{ text: string; errors: string[] }> {
  return await new Promise((resolve, reject) => {
    const child = spawn('codex', args, {
      env: process.env,
      stdio: ['ignore', 'pipe', 'pipe'],
    })

    let stdoutBuffer = ''
    let stderrBuffer = ''
    let textAccumulator = ''
    const errors: string[] = []

    const flushStdoutLine = (line: string) => {
      const event = parseCodexJsonLine(line)
      if (!event) return
      if (event.text) {
        textAccumulator += event.text
      }
      if (event.error) {
        errors.push(event.error)
      }
    }

    const timer = setTimeout(() => {
      child.kill('SIGTERM')
      reject(new Error(`Codex request timed out after ${Math.round(timeoutMs / 1000)}s.`))
    }, timeoutMs)

    child.stdout.on('data', (chunk: Buffer) => {
      stdoutBuffer += chunk.toString('utf-8')
      let idx = stdoutBuffer.indexOf('\n')
      while (idx >= 0) {
        const line = stdoutBuffer.slice(0, idx).trim()
        stdoutBuffer = stdoutBuffer.slice(idx + 1)
        if (line) flushStdoutLine(line)
        idx = stdoutBuffer.indexOf('\n')
      }
    })

    child.stderr.on('data', (chunk: Buffer) => {
      stderrBuffer += chunk.toString('utf-8')
    })

    child.on('error', (err) => {
      clearTimeout(timer)
      reject(err)
    })

    child.on('close', (code) => {
      clearTimeout(timer)

      const tail = stdoutBuffer.trim()
      if (tail) {
        flushStdoutLine(tail)
      }

      if (code === 0) {
        resolve({ text: textAccumulator, errors })
        return
      }

      const stderrError = extractCodexCliError(stderrBuffer)
      const fallback = errors[errors.length - 1]
      reject(
        new Error(
          stderrError
            || fallback
            || `Codex exited with code ${code ?? 'unknown'}.`,
        ),
      )
    })
  })
}

function parseCodexJsonLine(
  line: string,
): { text?: string; error?: string } | null {
  let parsed: Record<string, unknown>
  try {
    parsed = JSON.parse(line) as Record<string, unknown>
  } catch {
    return null
  }

  const type = typeof parsed.type === 'string' ? parsed.type : ''
  if (type === 'error') {
    const message = getStringField(parsed, ['message'])
    return { error: message || 'Codex returned an unknown error.' }
  }

  // Common Codex JSONL stream events include deltas in "delta" or "text".
  const text =
    getStringField(parsed, ['delta'])
    || getStringField(parsed, ['text'])
    || getStringField(parsed, ['content'])

  if (!text) return null
  return { text }
}

function getStringField(
  obj: Record<string, unknown>,
  keys: string[],
): string | null {
  for (const key of keys) {
    const val = obj[key]
    if (typeof val === 'string' && val.length > 0) {
      return val
    }
  }
  return null
}

function extractCodexCliError(stderr: string): string | null {
  const trimmed = stderr.trim()
  if (!trimmed) return null

  const lines = trimmed.split('\n').map((line) => line.trim()).filter(Boolean)
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i]
    if (line.toLowerCase().startsWith('error:')) {
      return line.replace(/^error:\s*/i, '').trim()
    }
  }

  return lines[lines.length - 1] ?? null
}
