import { defineEventHandler, readBody, setResponseHeaders, getQuery, createError } from 'h3'
import {
  createAgent,
  createTeam,
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

interface MemberDef {
  id: string
  providerType: 'anthropic' | 'openai-compat'
  apiKey: string
  model: string
  baseURL?: string
  systemPrompt?: string
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
  members?: MemberDef[]
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
    session.lastActivity = Date.now()
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

  // Create agent or team based on whether members are provided
  let agentOrTeam: { run: (msgs: any) => AsyncGenerator<any>; resolveToolResult: (id: string, result: any) => void }

  if (body.members?.length) {
    // Team mode — create member agents with scoped tools
    // Designer only gets generate_design + snapshot_layout (read-only check).
    // Giving all tools causes wasteful batch_get calls after generation.
    const DESIGNER_TOOLS = new Set(['generate_design', 'snapshot_layout'])

    const members = body.members.map(m => {
      const memberProvider = m.providerType === 'anthropic'
        ? createAnthropicProvider({ apiKey: m.apiKey, model: m.model, baseURL: m.baseURL })
        : createOpenAICompatProvider({ apiKey: m.apiKey, model: m.model, baseURL: m.baseURL })

      const memberTools = createToolRegistry()
      const allowedTools = m.id === 'designer' ? DESIGNER_TOOLS : null
      for (const def of body.toolDefs ?? []) {
        if (allowedTools && !allowedTools.has(def.name)) continue
        const params = def.parameters ? { ...def.parameters } : { type: 'object' }
        delete (params as any).$schema
        memberTools.register({
          name: def.name,
          description: def.description,
          level: def.level,
          schema: jsonSchema(params as any),
        })
      }

      return {
        id: m.id,
        provider: memberProvider,
        tools: memberTools,
        systemPrompt: m.systemPrompt || `You are a ${m.id} specialist.`,
        turnTimeout: 5 * 60_000, // 5 minutes — design generation is slow
      }
    })

    // Remove generate_design from lead tools — force delegation to designer
    const leadTools = createToolRegistry()
    for (const def of body.toolDefs ?? []) {
      if (def.name === 'generate_design') continue // designer-only
      const params = def.parameters ? { ...def.parameters } : { type: 'object' }
      delete (params as any).$schema
      leadTools.register({
        name: def.name,
        description: def.description,
        level: def.level,
        schema: jsonSchema(params as any),
      })
    }

    const team = createTeam({
      lead: { provider, tools: leadTools, systemPrompt: body.systemPrompt, maxTurns: body.maxTurns ?? 20 },
      members,
    })
    agentOrTeam = { run: (msgs) => team.run(msgs), resolveToolResult: (id, result) => team.resolveToolResult(id, result) }
  } else {
    const agent = createAgent({
      provider,
      tools,
      systemPrompt: body.systemPrompt,
      maxTurns: body.maxTurns ?? 20,
      maxOutputTokens: body.maxOutputTokens,
      turnTimeout: 5 * 60_000,
      abortSignal: abortController.signal,
    })
    agentOrTeam = agent
  }

  agentSessions.set(body.sessionId, { agent: agentOrTeam as any, abortController, createdAt: Date.now(), lastActivity: Date.now() })

  setResponseHeaders(event, {
    'Content-Type': 'text/event-stream',
    'Cache-Control': 'no-cache',
    Connection: 'keep-alive',
  })

  const encoder = new TextEncoder()
  let stream: ReadableStream
  try {
    stream = new ReadableStream({
      async start(controller) {
        const pingTimer = setInterval(() => {
          try {
            controller.enqueue(encoder.encode(': ping\n\n'))
          } catch { /* stream already closed */ }
        }, 5_000)

        try {
          for await (const agentEvent of agentOrTeam.run(toModelMessages(body.messages))) {
            const session = agentSessions.get(body.sessionId)
            if (session) session.lastActivity = Date.now()
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
  } catch (err) {
    // Stream construction failed — clean up session
    agentSessions.delete(body.sessionId)
    throw err
  }

  return new Response(stream)
})
