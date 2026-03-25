import { defineEventHandler, readBody, setResponseHeaders } from 'h3'
import {
  createAgent,
  createAnthropicProvider,
  createOpenAICompatProvider,
  createToolRegistry,
  encodeAgentEvent,
} from '@zseven-w/agent'
import type { AuthLevel } from '@zseven-w/agent'
import { z } from 'zod'
import { agentSessions } from '../../utils/agent-sessions'

interface ToolDef {
  name: string
  description: string
  level: AuthLevel
  jsonSchema?: Record<string, unknown>
}

interface AgentBody {
  sessionId: string
  messages: Array<{ role: string; content: unknown }>
  systemPrompt: string
  providerType: 'anthropic' | 'openai-compat'
  apiKey: string
  model: string
  baseURL?: string
  toolDefs: ToolDef[]
  maxTurns?: number
}

export default defineEventHandler(async (event) => {
  const body = await readBody<AgentBody>(event)
  if (!body?.sessionId || !body.messages || !body.systemPrompt || !body.providerType || !body.apiKey || !body.model) {
    setResponseHeaders(event, { 'Content-Type': 'application/json' })
    return { error: 'Missing required fields: sessionId, messages, systemPrompt, providerType, apiKey, model' }
  }

  // Create provider
  const provider = body.providerType === 'anthropic'
    ? createAnthropicProvider({ apiKey: body.apiKey, model: body.model })
    : createOpenAICompatProvider({ apiKey: body.apiKey, model: body.model, baseURL: body.baseURL })

  // Reconstruct tool registry from definitions (no execute — client-side execution)
  const tools = createToolRegistry()
  for (const def of body.toolDefs ?? []) {
    tools.register({
      name: def.name,
      description: def.description,
      level: def.level,
      schema: z.any(),
    })
  }

  const abortController = new AbortController()
  const agent = createAgent({
    provider,
    tools,
    systemPrompt: body.systemPrompt,
    maxTurns: body.maxTurns ?? 20,
    abortSignal: abortController.signal,
  })

  agentSessions.set(body.sessionId, { agent, abortController, createdAt: Date.now() })

  setResponseHeaders(event, {
    'Content-Type': 'text/event-stream',
    'Cache-Control': 'no-cache',
    Connection: 'keep-alive',
  })

  // SSE stream — follows the same ReadableStream pattern as chat.ts
  const encoder = new TextEncoder()
  const stream = new ReadableStream({
    async start(controller) {
      try {
        for await (const agentEvent of agent.run(body.messages as any)) {
          controller.enqueue(encoder.encode(encodeAgentEvent(agentEvent)))
        }
      } catch (err) {
        controller.enqueue(encoder.encode(encodeAgentEvent({
          type: 'error',
          message: String(err),
          fatal: true,
        })))
      } finally {
        agentSessions.delete(body.sessionId)
        controller.close()
      }
    },
  })

  return new Response(stream)
})
