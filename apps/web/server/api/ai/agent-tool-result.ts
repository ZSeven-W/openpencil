import { defineEventHandler, readBody, createError } from 'h3'
import { agentSessions } from '../../utils/agent-sessions'

interface ToolResultBody {
  sessionId: string
  toolCallId: string
  result: { success: boolean; data?: unknown; error?: string }
}

export default defineEventHandler(async (event) => {
  const body = await readBody<ToolResultBody>(event)
  if (!body?.sessionId || !body.toolCallId || !body.result) {
    throw createError({ statusCode: 400, message: 'Missing required fields: sessionId, toolCallId, result' })
  }

  const session = agentSessions.get(body.sessionId)
  if (!session) {
    throw createError({ statusCode: 404, message: 'Session not found' })
  }

  session.agent.resolveToolResult(body.toolCallId, body.result)
  return { ok: true }
})
