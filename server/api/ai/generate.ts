import { defineEventHandler, readBody, setResponseHeaders } from 'h3'

interface GenerateBody {
  system: string
  message: string
}

/**
 * Non-streaming AI generation endpoint.
 * Tries ANTHROPIC_API_KEY first (via Anthropic SDK);
 * falls back to local Claude Code (via Agent SDK, uses OAuth login).
 */
export default defineEventHandler(async (event) => {
  const body = await readBody<GenerateBody>(event)

  if (!body?.message || !body?.system) {
    setResponseHeaders(event, { 'Content-Type': 'application/json' })
    return { error: 'Missing required fields: system, message' }
  }

  const apiKey = process.env.ANTHROPIC_API_KEY
  if (apiKey) {
    try {
      return await generateViaAnthropicSDK(apiKey, body)
    } catch {
      // SDK not installed or failed — fall back to Agent SDK
    }
  }
  return generateViaAgentSDK(body)
})

/** Generate via Anthropic SDK */
async function generateViaAnthropicSDK(apiKey: string, body: GenerateBody) {
  try {
    // @ts-expect-error — optional dependency, only used when ANTHROPIC_API_KEY is set
    const { default: Anthropic } = await import('@anthropic-ai/sdk')
    const client = new Anthropic({ apiKey })
    const response = await client.messages.create({
      model: 'claude-sonnet-4-5-20250929',
      max_tokens: 4096,
      system: body.system,
      messages: [{ role: 'user', content: body.message }],
    })

    const textBlock = response.content.find((b: { type: string }) => b.type === 'text')
    return { text: textBlock?.text ?? '' }
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error'
    return { error: message }
  }
}

/** Generate via Claude Agent SDK (uses local Claude Code OAuth login, no API key needed) */
async function generateViaAgentSDK(body: GenerateBody): Promise<{ text?: string; error?: string }> {
  try {
    const { query } = await import('@anthropic-ai/claude-agent-sdk')

    // Remove CLAUDECODE env to allow running from within a CC terminal
    const env = { ...process.env } as Record<string, string | undefined>
    delete env.CLAUDECODE

    const q = query({
      prompt: body.message,
      options: {
        systemPrompt: body.system,
        model: 'claude-sonnet-4-6',
        maxTurns: 1,
        tools: [],
        permissionMode: 'plan',
        persistSession: false,
        env,
      },
    })

    for await (const message of q) {
      if (message.type === 'result') {
        if (message.subtype === 'success') {
          return { text: message.result }
        }
        const errors = 'errors' in message ? (message.errors as string[]) : []
        return { error: errors.join('; ') || `Query ended with: ${message.subtype}` }
      }
    }

    return { error: 'No result received from Claude Agent SDK' }
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error'
    return { error: message }
  }
}
