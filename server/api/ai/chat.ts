import { defineEventHandler, readBody, setResponseHeaders } from 'h3'

interface ChatBody {
  system: string
  messages: Array<{ role: 'user' | 'assistant'; content: string }>
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

  const apiKey = process.env.ANTHROPIC_API_KEY
  if (apiKey) {
    try {
      return await streamViaAnthropicSDK(apiKey, body)
    } catch {
      // SDK not installed or failed — fall back to Agent SDK
    }
  }
  return streamViaAgentSDK(body)
})

/** Stream via Anthropic SDK (when API key is available) */
async function streamViaAnthropicSDK(apiKey: string, body: ChatBody) {
  // @ts-expect-error — optional dependency, only used when ANTHROPIC_API_KEY is set
  const { default: Anthropic } = await import('@anthropic-ai/sdk')
  const client = new Anthropic({ apiKey })

  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder()
      try {
        const messageStream = client.messages.stream({
          model: 'claude-sonnet-4-5-20250929',
          max_tokens: 16384,
          system: body.system,
          messages: body.messages,
        })

        for await (const ev of messageStream) {
          if (
            ev.type === 'content_block_delta' &&
            ev.delta.type === 'text_delta'
          ) {
            const data = JSON.stringify({ type: 'text', content: ev.delta.text })
            controller.enqueue(encoder.encode(`data: ${data}\n\n`))
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
        controller.close()
      }
    },
  })

  return new Response(stream)
}

/** Stream via Claude Agent SDK (uses local Claude Code OAuth login, no API key needed) */
function streamViaAgentSDK(body: ChatBody) {
  const stream = new ReadableStream({
    async start(controller) {
      const encoder = new TextEncoder()

      try {
        const { query } = await import('@anthropic-ai/claude-agent-sdk')

        // Build prompt from the last user message
        const lastUserMsg = [...body.messages].reverse().find((m) => m.role === 'user')
        const prompt = lastUserMsg?.content ?? ''

        // Remove CLAUDECODE env to allow running from within a CC terminal
        const env = { ...process.env } as Record<string, string | undefined>
        delete env.CLAUDECODE

        const q = query({
          prompt,
          options: {
            systemPrompt: body.system,
            model: 'claude-sonnet-4-6',
            maxTurns: 1,
            includePartialMessages: true,
            tools: [],
            permissionMode: 'plan',
            persistSession: false,
            env,
          },
        })

        for await (const message of q) {
          if (message.type === 'stream_event') {
            const ev = message.event
            if (
              ev.type === 'content_block_delta' &&
              ev.delta.type === 'text_delta'
            ) {
              const data = JSON.stringify({ type: 'text', content: ev.delta.text })
              controller.enqueue(encoder.encode(`data: ${data}\n\n`))
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
        controller.close()
      }
    },
  })

  return new Response(stream)
}
