import { defineEventHandler, readBody, setResponseHeaders } from 'h3'
import { readFile, writeFile, mkdtemp, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { resolveClaudeCli } from '../../utils/resolve-claude-cli'
import { runCodexExec } from '../../utils/codex-client'
import {
  buildClaudeAgentEnv,
  getClaudeAgentDebugFilePath,
} from '../../utils/resolve-claude-agent-env'

interface ChatAttachmentWire {
  name: string
  mediaType: string
  data: string // base64
}

interface ChatBody {
  system: string
  messages: Array<{ role: 'user' | 'assistant'; content: string; attachments?: ChatAttachmentWire[] }>
  model?: string
  provider?: string
  thinkingMode?: 'adaptive' | 'disabled' | 'enabled'
  thinkingBudgetTokens?: number
  effort?: 'low' | 'medium' | 'high' | 'max'
}

async function readDebugTail(path?: string, maxLines = 40): Promise<string[] | undefined> {
  if (!path) return undefined
  try {
    const raw = await readFile(path, 'utf-8')
    const lines = raw.split('\n').filter((l) => l.trim().length > 0)
    return lines.slice(-maxLines)
  } catch {
    return undefined
  }
}

function shouldRetryClaudeWithoutModel(raw: string): boolean {
  return /process exited with code 1|invalid model|unknown model|model.*not/i.test(raw)
}

function buildClaudeExitHint(rawError: string, debugTail?: string[]): string | undefined {
  if (!/process exited with code 1/i.test(rawError)) return undefined
  if (!debugTail || debugTail.length === 0) return undefined
  const text = debugTail.join('\n')

  const hints: string[] = []
  if (/Failed to save config with lock: Error: EPERM|operation not permitted, .*\.claude\.json/i.test(text)) {
    hints.push('Claude Code cannot write ~/.claude.json in the current runtime (permission denied).')
  }
  if (/Connection error|Could not resolve host|Failed to connect/i.test(text)) {
    hints.push('Upstream API connection failed (check proxy/DNS/network reachability to your ANTHROPIC_BASE_URL).')
  }
  if (/ANTHROPIC_CUSTOM_HEADERS present: false, has Authorization header: false/i.test(text)) {
    hints.push('No API auth header detected by Claude runtime; verify token/header env mapping.')
  }

  if (hints.length === 0) return undefined
  return `${rawError}\n${hints.join(' ')}`
}

/**
 * Streaming chat endpoint.
 * Tries ANTHROPIC_API_KEY first (via Anthropic SDK);
 * falls back to local Claude Code (via Agent SDK, uses OAuth login).
 */
export default defineEventHandler(async (event) => {
  const body = await readBody<ChatBody>(event)

  if (!body?.messages || !body?.system) {
    setResponseHeaders(event, { 'Content-Type': 'application/json' })
    return { error: 'Missing required fields: system, messages' }
  }

  setResponseHeaders(event, {
    'Content-Type': 'text/event-stream',
    'Cache-Control': 'no-cache',
    Connection: 'keep-alive',
  })

  // Explicit provider routing
  if (body.provider === 'opencode') {
    return streamViaOpenCode(body, body.model)
  }
  if (body.provider === 'openai') {
    return streamViaCodex(body, body.model)
  }

  // Default: existing behavior (backward-compatible)
  const apiKey = process.env.ANTHROPIC_API_KEY
  if (apiKey) {
    try {
      return await streamViaAnthropicSDK(apiKey, body, body.model)
    } catch {
      // SDK not installed or failed — fall back to Agent SDK
    }
  }
  return streamViaAgentSDK(body, body.model)
})

// Keep-alive ping interval (ms) — prevents client timeout while waiting for API TTFT
const KEEPALIVE_INTERVAL_MS = 15_000
// Max time to wait for the first SDK event (text/thinking/error).
// If the API provider doesn't respond within this window, abort and surface
// a clear error instead of letting the client wait minutes for a timeout.
const API_CONNECT_TIMEOUT_MS = 30_000

function getAnthropicThinkingConfig(body: ChatBody):
  | { type: 'adaptive' | 'disabled' }
  | { type: 'enabled'; budget_tokens: number }
  | undefined {
  if (!body.thinkingMode) return undefined
  if (body.thinkingMode === 'enabled') {
    const budget = Math.max(1024, body.thinkingBudgetTokens ?? 1024)
    return { type: 'enabled', budget_tokens: budget }
  }
  return { type: body.thinkingMode }
}

function getAgentThinkingConfig(body: ChatBody):
  | { type: 'adaptive' | 'disabled' }
  | { type: 'enabled'; budgetTokens?: number }
  | undefined {
  if (!body.thinkingMode) return undefined
  if (body.thinkingMode === 'enabled') {
    return { type: 'enabled', budgetTokens: body.thinkingBudgetTokens }
  }
  return { type: body.thinkingMode }
}

/**
 * Save base64 attachments to temp files. Returns { tempDir, files[] } — caller must clean up tempDir.
 *
 * When `insideProject` is true, files are saved under `.openpencil-tmp/` in the
 * current working directory so that Claude Code Agent SDK (which restricts reads
 * to the project directory in plan mode) can access them.
 */
async function saveAttachmentsToTempFiles(
  attachments: ChatAttachmentWire[],
  insideProject = false,
): Promise<{ tempDir: string; files: string[] }> {
  let tempDir: string
  if (insideProject) {
    const { mkdirSync } = await import('node:fs')
    const baseDir = join(process.cwd(), '.openpencil-tmp')
    mkdirSync(baseDir, { recursive: true })
    tempDir = await mkdtemp(join(baseDir, 'attach-'))
  } else {
    tempDir = await mkdtemp(join(tmpdir(), 'openpencil-attach-'))
  }
  const files: string[] = []
  for (const att of attachments) {
    const ext = att.mediaType.split('/')[1] || 'png'
    const filePath = join(tempDir, `${files.length}.${ext}`)
    await writeFile(filePath, Buffer.from(att.data, 'base64'))
    files.push(filePath)
  }
  return { tempDir, files }
}

/** Collect all attachments from the last user message */
function getLastUserAttachments(body: ChatBody): ChatAttachmentWire[] {
  const lastUser = [...body.messages].reverse().find((m) => m.role === 'user')
  return lastUser?.attachments ?? []
}

/**
 * Strip "NEVER use tools" and similar instructions from system prompt
 * when we need Claude Code Agent SDK to use its Read tool for image analysis.
 */
function stripNoToolsRestriction(systemPrompt: string): string {
  return systemPrompt
    .replace(/^.*NEVER use tools.*$/gim, '')
    .replace(/\n{3,}/g, '\n\n')
}

/** Build Anthropic SDK multimodal messages from ChatBody messages */
function buildAnthropicMessages(body: ChatBody): Array<{ role: string; content: unknown }> {
  return body.messages.map((m) => {
    const attachments = m.attachments ?? []
    if (attachments.length === 0) {
      return { role: m.role, content: m.content }
    }
    const content: Array<Record<string, unknown>> = [
      ...attachments.map((a) => ({
        type: 'image',
        source: { type: 'base64', media_type: a.mediaType, data: a.data },
      })),
      { type: 'text', text: m.content || 'Analyze these images.' },
    ]
    return { role: m.role, content }
  })
}

/** Stream via Anthropic SDK (when API key is available) */
async function streamViaAnthropicSDK(apiKey: string, body: ChatBody, model?: string) {
  const { default: Anthropic } = await import('@anthropic-ai/sdk')
  // Disable automatic retries so auth/balance errors (429) surface immediately
  // instead of waiting through exponential backoff retry cycles.
  const client = new Anthropic({ apiKey, maxRetries: 0 })

  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder()
      const send = (type: string, content: string) => {
        try {
          controller.enqueue(encoder.encode(`data: ${JSON.stringify({ type, content })}\n\n`))
        } catch { /* stream already closed */ }
      }
      const pingTimer = setInterval(() => send('ping', ''), KEEPALIVE_INTERVAL_MS)

      // Abort if the API provider doesn't produce any event within the timeout.
      // Catches slow proxies, invalid keys on slow endpoints, etc.
      const connectAbort = new AbortController()
      let gotSdkEvent = false
      const connectTimer = setTimeout(() => {
        if (!gotSdkEvent) connectAbort.abort()
      }, API_CONNECT_TIMEOUT_MS)

      try {
        const thinking = getAnthropicThinkingConfig(body)
        const messageStream = client.messages.stream({
          model: model || 'claude-sonnet-4-5-20250929',
          max_tokens: 16384,
          system: body.system,
          messages: buildAnthropicMessages(body) as any,
          ...(body.effort ? { effort: body.effort } : {}),
          ...(thinking ? { thinking } : {}),
        }, { signal: connectAbort.signal })

        for await (const ev of messageStream) {
          if (!gotSdkEvent) {
            gotSdkEvent = true
            clearTimeout(connectTimer)
          }
          if (ev.type === 'content_block_delta') {
            if (ev.delta.type === 'text_delta') {
              clearInterval(pingTimer)
              send('text', ev.delta.text)
            } else if (ev.delta.type === 'thinking_delta') {
              send('thinking', ev.delta.thinking)
            }
          }
        }

        send('done', '')
      } catch (error) {
        clearTimeout(connectTimer)
        const content = connectAbort.signal.aborted && !gotSdkEvent
          ? 'API connection timed out (30s). Check your API key and network configuration.'
          : error instanceof Error ? error.message : 'Unknown error'
        send('error', content)
      } finally {
        clearTimeout(connectTimer)
        clearInterval(pingTimer)
        controller.close()
      }
    },
  })

  return new Response(stream)
}

/** Stream via Claude Agent SDK (uses local Claude Code OAuth login, no API key needed) */
function streamViaAgentSDK(body: ChatBody, model?: string) {
  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder()
      // Send keep-alive pings until the first real chunk arrives
      const pingTimer = setInterval(() => {
        try {
          controller.enqueue(encoder.encode(`data: ${JSON.stringify({ type: 'ping', content: '' })}\n\n`))
        } catch { /* stream already closed */ }
      }, KEEPALIVE_INTERVAL_MS)
      let emittedText = false
      let debugFile: string | undefined
      let attachTempDir: string | undefined

      try {
        const { query } = await import('@anthropic-ai/claude-agent-sdk')

        // Build prompt from the last user message
        const lastUserMsg = [...body.messages].reverse().find((m) => m.role === 'user')
        let prompt = lastUserMsg?.content ?? ''

        // If the last user message has image attachments, save to temp files
        // inside the project directory so Claude Code has read permission.
        const attachments = getLastUserAttachments(body)
        const hasImageAttachments = attachments.length > 0
        if (hasImageAttachments) {
          const saved = await saveAttachmentsToTempFiles(attachments, true)
          attachTempDir = saved.tempDir
          const imageRefs = saved.files.map((f) =>
            `First, use the Read tool to read the image file at "${f}". Then analyze it and respond to the user.`,
          ).join('\n')
          prompt = imageRefs + '\n\n' + (prompt || 'Describe what you see in the image.')
        }

        // Remove CLAUDECODE env to allow running from within a CC terminal
        const env = buildClaudeAgentEnv()
        debugFile = getClaudeAgentDebugFilePath()

        const claudePath = resolveClaudeCli()
        const thinking = getAgentThinkingConfig(body)

        // When images are attached, strip the "NEVER use tools" restriction from
        // the system prompt so Claude Code will use its Read tool to view images.
        const effectiveSystemPrompt = hasImageAttachments
          ? stripNoToolsRestriction(body.system)
          : body.system

        // When images are attached, use result-based flow (like validate.ts):
        // let Claude Code read the image via its Read tool internally, then
        // only emit the final result text. This avoids streaming intermediate
        // tool-use preamble like "I need to read the file first".
        if (hasImageAttachments) {
          const runImageQuery = async (modelOverride?: string): Promise<string> => {
            const q = query({
              prompt,
              options: {
                systemPrompt: effectiveSystemPrompt,
                ...(modelOverride ? { model: modelOverride } : {}),
                maxTurns: 3,
                plugins: [],
                permissionMode: 'plan',
                persistSession: false,
                ...(body.effort ? { effort: body.effort } : {}),
                ...(thinking ? { thinking } : {}),
                env,
                ...(debugFile ? { debugFile } : {}),
                ...(claudePath ? { pathToClaudeCodeExecutable: claudePath } : {}),
              },
            })

            try {
              for await (const message of q) {
                if (message.type === 'result') {
                  const isErrorResult = 'is_error' in message && Boolean((message as { is_error?: boolean }).is_error)
                  if (message.subtype === 'success' && !isErrorResult) {
                    return message.result ?? ''
                  }
                  const errors = 'errors' in message ? (message.errors as string[]) : []
                  const resultText = 'result' in message ? String(message.result ?? '') : ''
                  const errContent = errors.join('; ') || resultText || `Query ended with: ${message.subtype}`
                  if (modelOverride && shouldRetryClaudeWithoutModel(errContent)) {
                    throw new Error(errContent)
                  }
                  throw new Error(errContent)
                }
              }
              return ''
            } finally {
              q.close()
            }
          }

          let resultText: string
          try {
            resultText = await runImageQuery(model)
          } catch (error) {
            const raw = error instanceof Error ? error.message : String(error)
            if (model && shouldRetryClaudeWithoutModel(raw)) {
              resultText = await runImageQuery(undefined)
            } else {
              throw error
            }
          }

          clearInterval(pingTimer)
          if (resultText) {
            emittedText = true
            controller.enqueue(
              encoder.encode(`data: ${JSON.stringify({ type: 'text', content: resultText })}\n\n`),
            )
          }
        } else {
          // Normal text-only chat: stream partial messages as before
          const runQuery = async (modelOverride?: string) => {
            const q = query({
              prompt,
              options: {
                systemPrompt: effectiveSystemPrompt,
                ...(modelOverride ? { model: modelOverride } : {}),
                maxTurns: 1,
                includePartialMessages: true,
                tools: [],
                plugins: [],
                permissionMode: 'plan',
                persistSession: false,
                ...(body.effort ? { effort: body.effort } : {}),
                ...(thinking ? { thinking } : {}),
                env,
                ...(debugFile ? { debugFile } : {}),
                ...(claudePath ? { pathToClaudeCodeExecutable: claudePath } : {}),
              },
            })

            try {
              for await (const message of q) {
                if (message.type === 'stream_event') {
                  const ev = message.event
                  if (ev.type === 'content_block_delta') {
                    if (ev.delta.type === 'text_delta') {
                      emittedText = true
                      clearInterval(pingTimer)
                      const data = JSON.stringify({ type: 'text', content: ev.delta.text })
                      controller.enqueue(encoder.encode(`data: ${data}\n\n`))
                    } else if (ev.delta.type === 'thinking_delta') {
                      // Keep pings alive during thinking — only stop on text output
                      const data = JSON.stringify({ type: 'thinking', content: (ev.delta as any).thinking })
                      controller.enqueue(encoder.encode(`data: ${data}\n\n`))
                    }
                  }
                } else if (message.type === 'result') {
                  const isErrorResult = 'is_error' in message && Boolean((message as { is_error?: boolean }).is_error)
                  if (message.subtype !== 'success' || isErrorResult) {
                    const errors = 'errors' in message ? (message.errors as string[]) : []
                    const resultText = 'result' in message ? String(message.result ?? '') : ''
                    const content = errors.join('; ') || resultText || `Query ended with: ${message.subtype}`
                    if (modelOverride && !emittedText && shouldRetryClaudeWithoutModel(content)) {
                      throw new Error(content)
                    }
                    controller.enqueue(
                      encoder.encode(`data: ${JSON.stringify({ type: 'error', content })}\n\n`),
                    )
                  }
                }
              }
            } finally {
              q.close()
            }
          }

          try {
            await runQuery(model)
          } catch (error) {
            const raw = error instanceof Error ? error.message : String(error)
            if (model && !emittedText && shouldRetryClaudeWithoutModel(raw)) {
              await runQuery(undefined)
            } else {
              throw error
            }
          }
        }

        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'done', content: '' })}\n\n`),
        )
      } catch (error) {
        const rawContent = error instanceof Error ? error.message : 'Unknown error'
        const tail = await readDebugTail(debugFile)
        const hintedContent = buildClaudeExitHint(rawContent, tail)
        const content = hintedContent ?? rawContent
        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'error', content })}\n\n`),
        )
      } finally {
        clearInterval(pingTimer)
        if (attachTempDir) {
          rm(attachTempDir, { recursive: true, force: true }).catch(() => {})
        }
        controller.close()
      }
    },
  })

  return new Response(stream)
}

/** Parse an OpenCode model string ("providerID/modelID") into its parts */
function parseOpenCodeModel(model?: string): { providerID: string; modelID: string } | undefined {
  if (!model || !model.includes('/')) return undefined
  const idx = model.indexOf('/')
  return { providerID: model.slice(0, idx), modelID: model.slice(idx + 1) }
}

function mapOpenCodeEffort(
  effort?: 'low' | 'medium' | 'high' | 'max',
): 'low' | 'medium' | 'high' | undefined {
  if (!effort) return undefined
  if (effort === 'max') return 'high'
  return effort
}

function buildOpenCodeReasoning(
  body: ChatBody,
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

async function promptOpenCodeWithThinking(
  ocClient: any,
  basePayload: Record<string, unknown>,
  body: ChatBody,
): Promise<{ data: any; error: any }> {
  const reasoning = buildOpenCodeReasoning(body)
  if (!reasoning) {
    return await ocClient.session.prompt(basePayload)
  }

  const enhanced = { ...basePayload, reasoning }
  const firstTry = await ocClient.session.prompt(enhanced)
  if (!firstTry.error) {
    return firstTry
  }

  console.warn('[AI] OpenCode reasoning options rejected, retrying without reasoning.')
  return await ocClient.session.prompt(basePayload)
}

function streamViaCodex(body: ChatBody, model?: string) {
  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder()
      const pingTimer = setInterval(() => {
        try {
          controller.enqueue(encoder.encode(`data: ${JSON.stringify({ type: 'ping', content: '' })}\n\n`))
        } catch { /* stream already closed */ }
      }, KEEPALIVE_INTERVAL_MS)

      let attachTempDir: string | undefined
      try {
        const lastUserMsg = [...body.messages].reverse().find((m) => m.role === 'user')
        const prompt = lastUserMsg?.content ?? ''

        // Save image attachments to temp files for Codex CLI
        const attachments = getLastUserAttachments(body)
        let imageFiles: string[] | undefined
        if (attachments.length > 0) {
          const saved = await saveAttachmentsToTempFiles(attachments)
          attachTempDir = saved.tempDir
          imageFiles = saved.files
        }

        const result = await runCodexExec(prompt, {
          model,
          systemPrompt: body.system,
          thinkingMode: body.thinkingMode,
          thinkingBudgetTokens: body.thinkingBudgetTokens,
          effort: body.effort,
          imageFiles,
        })

        clearInterval(pingTimer)
        if (result.error) {
          controller.enqueue(
            encoder.encode(`data: ${JSON.stringify({ type: 'error', content: result.error })}\n\n`),
          )
          return
        }

        if (result.text) {
          controller.enqueue(
            encoder.encode(`data: ${JSON.stringify({ type: 'text', content: result.text })}\n\n`),
          )
        }

        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'done', content: '' })}\n\n`),
        )
      } catch (error) {
        const content = error instanceof Error ? error.message : 'Unknown error'
        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'error', content })}\n\n`),
        )
      } finally {
        clearInterval(pingTimer)
        if (attachTempDir) {
          rm(attachTempDir, { recursive: true, force: true }).catch(() => {})
        }
        controller.close()
      }
    },
  })

  return new Response(stream)
}

/** Stream via OpenCode SDK (connects to a running OpenCode server) */
function streamViaOpenCode(body: ChatBody, model?: string) {
  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder()
      const pingTimer = setInterval(() => {
        try {
          controller.enqueue(encoder.encode(`data: ${JSON.stringify({ type: 'ping', content: '' })}\n\n`))
        } catch { /* stream already closed */ }
      }, KEEPALIVE_INTERVAL_MS)

      let ocServer: { close(): void } | undefined
      try {
        const { getOpencodeClient } = await import('../../utils/opencode-client')
        const oc = await getOpencodeClient()
        const ocClient = oc.client
        ocServer = oc.server

        // Create a session for this conversation
        const { data: session, error: sessionError } = await ocClient.session.create({
          title: 'OpenPencil Chat',
        })
        if (sessionError || !session) {
          throw new Error('Failed to create OpenCode session')
        }

        // Inject system prompt as context (no AI reply)
        await ocClient.session.prompt({
          sessionID: session.id,
          noReply: true,
          parts: [{ type: 'text', text: body.system }],
        })

        // Build prompt from the last user message
        const lastUserMsg = [...body.messages].reverse().find((m) => m.role === 'user')
        const prompt = lastUserMsg?.content ?? ''

        const parsed = parseOpenCodeModel(model)

        // Build parts array, adding image attachments if present
        const attachments = getLastUserAttachments(body)
        const parts: Array<Record<string, unknown>> = [
          ...attachments.map((a) => ({
            type: 'image',
            url: `data:${a.mediaType};base64,${a.data}`,
          })),
          { type: 'text', text: prompt || 'Analyze these images.' },
        ]

        // Send prompt and await full response
        const promptPayload: Record<string, unknown> = {
          sessionID: session.id,
          ...(parsed ? { model: parsed } : {}),
          parts,
        }

        const { data: result, error: promptError } = await promptOpenCodeWithThinking(
          ocClient,
          promptPayload,
          body,
        )

        if (promptError) {
          throw new Error('OpenCode prompt failed')
        }

        // Extract text from response parts
        clearInterval(pingTimer)
        if (result?.parts) {
          for (const part of result.parts) {
            if (part.type === 'text' && 'text' in part) {
              const data = JSON.stringify({ type: 'text', content: part.text })
              controller.enqueue(encoder.encode(`data: ${data}\n\n`))
            }
          }
        }

        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'done', content: '' })}\n\n`),
        )
      } catch (error) {
        const content = error instanceof Error ? error.message : 'Unknown error'
        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'error', content })}\n\n`),
        )
      } finally {
        const { releaseOpencodeServer } = await import('../../utils/opencode-client')
        releaseOpencodeServer(ocServer)
        clearInterval(pingTimer)
        controller.close()
      }
    },
  })

  return new Response(stream)
}
