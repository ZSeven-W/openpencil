import { defineEventHandler, readBody, setResponseHeaders, getQuery, createError } from 'h3'
import {
  createAgent,
  createAnthropicProvider,
  createOpenAICompatProvider,
  createToolRegistry,
  encodeAgentEvent,
} from '@zseven-w/agent'
import type { AuthLevel } from '@zseven-w/agent'
import { jsonSchema } from '@zseven-w/agent'
import { agentSessions } from '../../utils/agent-sessions'

interface ToolDef {
  name: string
  description: string
  level: AuthLevel
  /** JSON Schema from client — single source of truth, no server-side duplication */
  parameters?: Record<string, unknown>
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
  maxOutputTokens?: number
}

function toModelMessages(raw: Array<{ role: string; content: unknown }>) {
  return raw
    .filter((m) => (m.role === 'user' || m.role === 'assistant') && typeof m.content === 'string')
    .map((m) => ({
      role: m.role as 'user' | 'assistant',
      content: m.content as string,
    }))
}

/**
 * Unified agent endpoint. Routes by `?action=` query param:
 *   POST /api/ai/agent              — Start agent loop (SSE stream)
 *   POST /api/ai/agent?action=result — Resolve a pending tool call
 *   POST /api/ai/agent?action=abort  — Abort an agent session
 */
export default defineEventHandler(async (event) => {
  const { action } = getQuery(event) as { action?: string }

  // ── Tool result callback ────────────────────────────────────
  if (action === 'result') {
    const body = await readBody<{ sessionId: string; toolCallId: string; result: any }>(event)
    if (!body?.sessionId || !body.toolCallId || !body.result) {
      throw createError({ statusCode: 400, message: 'Missing: sessionId, toolCallId, result' })
    }
    const session = agentSessions.get(body.sessionId)
    if (!session) {
      throw createError({ statusCode: 404, message: 'Session not found' })
    }
    session.agent.resolveToolResult(body.toolCallId, body.result)
    return { ok: true }
  }

  // ── Abort ───────────────────────────────────────────────────
  if (action === 'abort') {
    const body = await readBody<{ sessionId?: string }>(event)
    const sid = body?.sessionId
    if (sid) {
      const session = agentSessions.get(sid)
      if (session) {
        session.abortController.abort()
        agentSessions.delete(sid)
      }
    }
    return { ok: true }
  }

  // ── Start agent loop (SSE stream) ──────────────────────────
  const body = await readBody<AgentBody>(event)
  if (!body?.sessionId || !body.messages || !body.systemPrompt || !body.providerType || !body.apiKey || !body.model) {
    setResponseHeaders(event, { 'Content-Type': 'application/json' })
    return { error: 'Missing required fields: sessionId, messages, systemPrompt, providerType, apiKey, model' }
  }

  const provider = body.providerType === 'anthropic'
    ? createAnthropicProvider({ apiKey: body.apiKey, model: body.model, baseURL: body.baseURL })
    : createOpenAICompatProvider({ apiKey: body.apiKey, model: body.model, baseURL: body.baseURL })

  const tools = createToolRegistry()
  for (const def of body.toolDefs ?? []) {
    // Use client-provided JSON Schema (single source of truth)
    // Strip $schema field that strict APIs (MiniMax, StepFun) reject
    const params = def.parameters ? { ...def.parameters } : { type: 'object' }
    delete (params as any).$schema
    tools.register({
      name: def.name,
      description: def.description,
      level: def.level,
      schema: jsonSchema(params as any),
    })
  }

  const abortController = new AbortController()
  const agent = createAgent({
    provider,
    tools,
    systemPrompt: body.systemPrompt,
    maxTurns: body.maxTurns ?? 20,
    maxOutputTokens: body.maxOutputTokens,
    turnTimeout: 5 * 60_000, // 5 minutes — generate_design runs the full orchestrator pipeline
    abortSignal: abortController.signal,
  })

  agentSessions.set(body.sessionId, { agent, abortController, createdAt: Date.now() })

  setResponseHeaders(event, {
    'Content-Type': 'text/event-stream',
    'Cache-Control': 'no-cache',
    Connection: 'keep-alive',
  })

  const encoder = new TextEncoder()
  const stream = new ReadableStream({
    async start(controller) {
      // Keep-alive ping every 5s — prevents Bun's 10s idle timeout from killing the SSE stream
      // while the agent loop is suspended waiting for tool results
      const pingTimer = setInterval(() => {
        try {
          controller.enqueue(encoder.encode(': ping\n\n'))
        } catch { /* stream already closed */ }
      }, 5_000)

      try {
        for await (const agentEvent of agent.run(toModelMessages(body.messages))) {
          controller.enqueue(encoder.encode(encodeAgentEvent(agentEvent)))
        }
      } catch (err: any) {
        try {
          controller.enqueue(encoder.encode(encodeAgentEvent({
            type: 'error',
            message: err?.message ?? String(err),
            fatal: true,
          })))
        } catch { /* ignore */ }
      } finally {
        clearInterval(pingTimer)
        agentSessions.delete(body.sessionId)
        try { controller.close() } catch { /* ignore */ }
      }
    },
  })

  return new Response(stream)
})
