import { defineEventHandler, readBody, setResponseHeaders } from 'h3'
import { resolveClaudeCli } from '../../utils/resolve-claude-cli'
import { runCodexExec } from '../../utils/codex-client'

interface ChatBody {
  system: string
  messages: Array<{ role: 'user' | 'assistant'; content: string }>
  model?: string
  provider?: string
  thinkingMode?: 'adaptive' | 'disabled' | 'enabled'
  thinkingBudgetTokens?: number
  effort?: 'low' | 'medium' | 'high' | 'max'
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

/** Stream via Anthropic SDK (when API key is available) */
async function streamViaAnthropicSDK(apiKey: string, body: ChatBody, model?: string) {
  const { default: Anthropic } = await import('@anthropic-ai/sdk')
  const client = new Anthropic({ apiKey })

  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder()
      // Send keep-alive pings until the first real chunk arrives
      const pingTimer = setInterval(() => {
        try {
          controller.enqueue(encoder.encode(`data: ${JSON.stringify({ type: 'ping', content: '' })}\n\n`))
        } catch { /* stream already closed */ }
      }, KEEPALIVE_INTERVAL_MS)
      try {
        const thinking = getAnthropicThinkingConfig(body)
        const messageStream = client.messages.stream({
          model: model || 'claude-sonnet-4-5-20250929',
          max_tokens: 16384,
          system: body.system,
          messages: body.messages,
          ...(body.effort ? { effort: body.effort } : {}),
          ...(thinking ? { thinking } : {}),
        })

        for await (const ev of messageStream) {
          if (ev.type === 'content_block_delta') {
            if (ev.delta.type === 'text_delta') {
              clearInterval(pingTimer)
              const data = JSON.stringify({ type: 'text', content: ev.delta.text })
              controller.enqueue(encoder.encode(`data: ${data}\n\n`))
            } else if (ev.delta.type === 'thinking_delta') {
              // Keep pings alive during thinking — only stop on text output
              const data = JSON.stringify({ type: 'thinking', content: ev.delta.thinking })
              controller.enqueue(encoder.encode(`data: ${data}\n\n`))
            }
          }
        }

        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'done', content: '' })}\n\n`),
        )
      } catch (error) {
        const msg = error instanceof Error ? error.message : 'Unknown error'
        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: 'error', content: msg })}\n\n`),
        )
      } finally {
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

      try {
        const { query } = await import('@anthropic-ai/claude-agent-sdk')

        // Build prompt from the last user message
        const lastUserMsg = [...body.messages].reverse().find((m) => m.role === 'user')
        let prompt = lastUserMsg?.content ?? ''

        // Remove CLAUDECODE env to allow running from within a CC terminal
        const env = { ...process.env } as Record<string, string | undefined>
        delete env.CLAUDECODE

        const claudePath = resolveClaudeCli()
        const thinking = getAgentThinkingConfig(body)

        const q = query({
          prompt,
          options: {
            systemPrompt: body.system,
            model: model || 'claude-sonnet-4-6',
            maxTurns: 1,
            includePartialMessages: true,
            tools: [],
            plugins: [],
            permissionMode: 'plan',
            persistSession: false,
            ...(body.effort ? { effort: body.effort } : {}),
            ...(thinking ? { thinking } : {}),
            env,
            ...(claudePath ? { pathToClaudeCodeExecutable: claudePath } : {}),
          },
        })

        for await (const message of q) {
          if (message.type === 'stream_event') {
            const ev = message.event
            if (ev.type === 'content_block_delta') {
              if (ev.delta.type === 'text_delta') {
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
            if (message.subtype !== 'success') {
              const errors = 'errors' in message ? (message.errors as string[]) : []
              const content = errors.join('; ') || `Query ended with: ${message.subtype}`
              controller.enqueue(
                encoder.encode(`data: ${JSON.stringify({ type: 'error', content })}\n\n`),
              )
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
        clearInterval(pingTimer)
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

      try {
        const lastUserMsg = [...body.messages].reverse().find((m) => m.role === 'user')
        const prompt = lastUserMsg?.content ?? ''
        const result = await runCodexExec(prompt, {
          model,
          systemPrompt: body.system,
          thinkingMode: body.thinkingMode,
          thinkingBudgetTokens: body.thinkingBudgetTokens,
          effort: body.effort,
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

        // Send prompt and await full response
        const promptPayload: Record<string, unknown> = {
          sessionID: session.id,
          ...(parsed ? { model: parsed } : {}),
          parts: [{ type: 'text', text: prompt }],
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
