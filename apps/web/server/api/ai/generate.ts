import { defineEventHandler, readBody, setResponseHeaders } from 'h3'
import { resolveClaudeCli } from '../../utils/resolve-claude-cli'
import { runCodexExec } from '../../utils/codex-client'
import {
  buildClaudeAgentEnv,
  buildSpawnClaudeCodeProcess,
  getClaudeAgentDebugFilePath,
  resolveAgentModel,
} from '../../utils/resolve-claude-agent-env'
import { formatOpenCodeError } from './chat'

interface GenerateBody {
  system: string
  message: string
  model?: string
  provider?: 'anthropic' | 'openai' | 'opencode' | 'gemini'
  thinkingMode?: 'adaptive' | 'disabled' | 'enabled'
  thinkingBudgetTokens?: number
  effort?: 'low' | 'medium' | 'high' | 'max'
}

/**
 * Non-streaming AI generation endpoint.
 * Routes to the appropriate provider SDK based on the `provider` field.
 * Requires explicit provider and model; no fallback routing.
 */
export default defineEventHandler(async (event) => {
  const body = await readBody<GenerateBody>(event)

  if (!body?.message || body?.system == null) {
    setResponseHeaders(event, { 'Content-Type': 'application/json' })
    return { error: 'Missing required fields: system, message' }
  }
  if (!body.provider) {
    setResponseHeaders(event, { 'Content-Type': 'application/json' })
    return { error: 'Missing provider. Provider fallback is disabled.' }
  }
  if (!body.model?.trim()) {
    setResponseHeaders(event, { 'Content-Type': 'application/json' })
    return { error: 'Missing model. Model fallback is disabled.' }
  }

  if (body.provider === 'anthropic') {
    return generateViaAgentSDK(body, body.model)
  }
  if (body.provider === 'opencode') {
    return generateViaOpenCode(body, body.model)
  }
  if (body.provider === 'openai') {
    return generateViaCodex(body, body.model)
  }
  if (body.provider === 'gemini') {
    return generateViaGemini(body, body.model)
  }
  return { error: 'Missing or unsupported provider. Provider fallback is disabled.' }
})

/** Generate via Claude Agent SDK (uses local Claude Code OAuth login, no API key needed) */
async function generateViaAgentSDK(body: GenerateBody, requestedModel?: string): Promise<{ text?: string; error?: string }> {
  const runQuery = async (): Promise<{ text?: string; error?: string }> => {
    const { query } = await import('@anthropic-ai/claude-agent-sdk')

    // Remove CLAUDECODE env to allow running from within a CC terminal
    const env = buildClaudeAgentEnv()
    const debugFile = getClaudeAgentDebugFilePath()
    const model = resolveAgentModel(requestedModel, env)

    const claudePath = resolveClaudeCli()

    const q = query({
      prompt: body.message,
      options: {
        systemPrompt: body.system,
        ...(model ? { model } : {}),
        maxTurns: 1,
        tools: [],
        plugins: [],
        permissionMode: 'plan',
        persistSession: false,
        env,
        ...(debugFile ? { debugFile } : {}),
        ...(claudePath ? { pathToClaudeCodeExecutable: claudePath } : {}),
        ...(buildSpawnClaudeCodeProcess() ? { spawnClaudeCodeProcess: buildSpawnClaudeCodeProcess() } : {}),
      },
    })

    try {
      for await (const message of q) {
        if (message.type === 'result') {
          const isErrorResult = 'is_error' in message && Boolean((message as { is_error?: boolean }).is_error)
          if (message.subtype === 'success' && !isErrorResult) {
            return { text: message.result }
          }
          const errors = 'errors' in message ? (message.errors as string[]) : []
          const resultText = 'result' in message ? String(message.result ?? '') : ''
          return { error: errors.join('; ') || resultText || `Query ended with: ${message.subtype}` }
        }
      }
    } finally {
      q.close()
    }

    return { error: 'No result received from Claude Agent SDK' }
  }

  try {
    return await runQuery()
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    return { error: message }
  }
}

async function generateViaCodex(body: GenerateBody, model?: string): Promise<{ text?: string; error?: string }> {
  const result = await runCodexExec(body.message, {
    model,
    systemPrompt: body.system,
    thinkingMode: body.thinkingMode,
    thinkingBudgetTokens: body.thinkingBudgetTokens,
    effort: body.effort,
  })
  return result.error ? { error: result.error } : { text: result.text ?? '' }
}

function mapOpenCodeEffort(
  effort?: 'low' | 'medium' | 'high' | 'max',
): 'low' | 'medium' | 'high' | undefined {
  if (!effort) return undefined
  if (effort === 'max') return 'high'
  return effort
}

function buildOpenCodeReasoning(
  body: GenerateBody,
): Record<string, unknown> | undefined {
  const reasoning: Record<string, unknown> = {}
  const effort = mapOpenCodeEffort(body.effort)
  if (effort) {
    reasoning.effort = effort
  }
  if (body.thinkingMode === 'enabled') {
    reasoning.enabled = true
  } else if (body.thinkingMode === 'disabled') {
    reasoning.enabled = false
  }
  if (typeof body.thinkingBudgetTokens === 'number' && body.thinkingBudgetTokens > 0) {
    reasoning.budgetTokens = body.thinkingBudgetTokens
  }
  return Object.keys(reasoning).length > 0 ? reasoning : undefined
}

/** Timeout for OpenCode prompt calls (3 minutes) */
const OPENCODE_PROMPT_TIMEOUT_MS = 180_000

async function promptWithTimeout(
  ocClient: any,
  payload: Record<string, unknown>,
  timeoutMs = OPENCODE_PROMPT_TIMEOUT_MS,
): Promise<{ data: any; error: any }> {
  const result = await Promise.race([
    ocClient.session.prompt(payload),
    new Promise<{ data: null; error: string }>((resolve) =>
      setTimeout(
        () => resolve({ data: null, error: `OpenCode prompt timed out after ${timeoutMs / 1000}s` }),
        timeoutMs,
      ),
    ),
  ])
  return result
}

async function promptOpenCodeWithThinking(
  ocClient: any,
  basePayload: Record<string, unknown>,
  body: GenerateBody,
): Promise<{ data: any; error: any }> {
  const reasoning = buildOpenCodeReasoning(body)
  if (!reasoning) {
    return await promptWithTimeout(ocClient, basePayload)
  }

  const enhanced = { ...basePayload, reasoning }
  const firstTry = await promptWithTimeout(ocClient, enhanced)
  if (!firstTry.error) {
    return firstTry
  }

  console.warn('[AI] OpenCode reasoning options rejected, retrying without reasoning.')
  return await promptWithTimeout(ocClient, basePayload)
}

/** Generate via OpenCode SDK (connects to a running OpenCode server) */
async function generateViaOpenCode(body: GenerateBody, model?: string): Promise<{ text?: string; error?: string }> {
  let ocServer: { close(): void } | undefined
  try {
    const { getOpencodeClient } = await import('../../utils/opencode-client')
    const oc = await getOpencodeClient()
    const ocClient = oc.client
    ocServer = oc.server

    const { data: session, error: sessionError } = await ocClient.session.create({
      title: 'OpenPencil Generate',
    })
    if (sessionError || !session) {
      const detail = formatOpenCodeError(sessionError)
      return { error: `Failed to create OpenCode session: ${detail}` }
    }

    // Inject system prompt as context (no AI reply)
    await ocClient.session.prompt({
      sessionID: session.id,
      noReply: true,
      parts: [{ type: 'text', text: body.system }],
    })

    // Parse model string ("providerID/modelID")
    let modelOption: { providerID: string; modelID: string } | undefined
    if (model && model.includes('/')) {
      const idx = model.indexOf('/')
      modelOption = { providerID: model.slice(0, idx), modelID: model.slice(idx + 1) }
    } else if (model) {
      console.warn(`[AI] OpenCode generate: could not parse model string "${model}", sending without model override`)
    }

    // Send main prompt and await full response
    const promptPayload: Record<string, unknown> = {
      sessionID: session.id,
      ...(modelOption ? { model: modelOption } : {}),
      parts: [{ type: 'text', text: body.message }],
    }

    console.log(`[AI] OpenCode generate: model=${model}, parsed=${JSON.stringify(modelOption)}`)

    const { data: result, error: promptError } = await promptOpenCodeWithThinking(
      ocClient,
      promptPayload,
      body,
    )

    if (promptError) {
      const errorDetail = formatOpenCodeError(promptError)
      console.error('[AI] OpenCode generate error:', errorDetail)
      return { error: errorDetail }
    }

    // Extract text from response parts
    const texts: string[] = []
    if (result?.parts) {
      for (const part of result.parts) {
        if (part.type === 'text' && part.text) {
          texts.push(part.text)
        }
      }
    }

    if (texts.length === 0) {
      return { error: 'OpenCode returned an empty response. The model may not have generated any output.' }
    }

    return { text: texts.join('') }
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error'
    return { error: message }
  } finally {
    const { releaseOpencodeServer } = await import('../../utils/opencode-client')
    releaseOpencodeServer(ocServer)
  }
}

/** Generate via Gemini CLI (`gemini -p -o json`) — CLI handles its own auth */
async function generateViaGemini(body: GenerateBody, model?: string): Promise<{ text?: string; error?: string }> {
  const { runGeminiExec } = await import('../../utils/gemini-client')
  return runGeminiExec(body.message, {
    model,
    systemPrompt: body.system,
    thinkingMode: body.thinkingMode,
    thinkingBudgetTokens: body.thinkingBudgetTokens,
    effort: body.effort,
  })
}
