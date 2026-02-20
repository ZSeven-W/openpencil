import { defineEventHandler, readBody, setResponseHeaders } from 'h3'

interface GenerateBody {
  system: string
  message: string
  model?: string
  provider?: string
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

  // Explicit provider routing
  if (body.provider === 'opencode') {
    return generateViaOpenCode(body, body.model)
  }

  // Default: existing behavior (backward-compatible)
  const apiKey = process.env.ANTHROPIC_API_KEY
  if (apiKey) {
    try {
      return await generateViaAnthropicSDK(apiKey, body, body.model)
    } catch {
      // SDK not installed or failed — fall back to Agent SDK
    }
  }
  return generateViaAgentSDK(body, body.model)
})

/** Generate via Anthropic SDK */
async function generateViaAnthropicSDK(apiKey: string, body: GenerateBody, model?: string) {
  try {
    const { default: Anthropic } = await import('@anthropic-ai/sdk')
    const client = new Anthropic({ apiKey })
    const response = await client.messages.create({
      model: model || 'claude-sonnet-4-5-20250929',
      max_tokens: 4096,
      system: body.system,
      messages: [{ role: 'user', content: body.message }],
    })

    const textBlock = response.content.find((b: { type: string }) => b.type === 'text')
    return { text: textBlock && 'text' in textBlock ? textBlock.text : '' }
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error'
    return { error: message }
  }
}

/** Generate via Claude Agent SDK (uses local Claude Code OAuth login, no API key needed) */
async function generateViaAgentSDK(body: GenerateBody, model?: string): Promise<{ text?: string; error?: string }> {
  try {
    const { query } = await import('@anthropic-ai/claude-agent-sdk')

    // Remove CLAUDECODE env to allow running from within a CC terminal
    const env = { ...process.env } as Record<string, string | undefined>
    delete env.CLAUDECODE

    const q = query({
      prompt: body.message,
      options: {
        systemPrompt: body.system,
        model: model || 'claude-sonnet-4-6',
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

/** Generate via OpenCode SDK (connects to a running OpenCode server) */
async function generateViaOpenCode(body: GenerateBody, model?: string): Promise<{ text?: string; error?: string }> {
  let ocServer: { close(): void } | undefined
  try {
    const { createOpencode } = await import('@opencode-ai/sdk/v2')
    const oc = await createOpencode()
    ocServer = oc.server

    const { data: session, error: sessionError } = await oc.client.session.create({
      title: 'OpenPencil Generate',
    })
    if (sessionError || !session) {
      return { error: 'Failed to create OpenCode session' }
    }

    // Inject system prompt as context (no AI reply)
    await oc.client.session.prompt({
      sessionID: session.id,
      noReply: true,
      parts: [{ type: 'text', text: body.system }],
    })

    // Parse model string ("providerID/modelID")
    let modelOption: { providerID: string; modelID: string } | undefined
    if (model && model.includes('/')) {
      const idx = model.indexOf('/')
      modelOption = { providerID: model.slice(0, idx), modelID: model.slice(idx + 1) }
    }

    // Send main prompt and await full response
    const { data: result, error: promptError } = await oc.client.session.prompt({
      sessionID: session.id,
      ...(modelOption ? { model: modelOption } : {}),
      parts: [{ type: 'text', text: body.message }],
    })

    if (promptError) {
      return { error: 'OpenCode generation failed' }
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

    return { text: texts.join('') }
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error'
    return { error: message }
  } finally {
    ocServer?.close()
  }
}
