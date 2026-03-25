import { defineEventHandler, readBody } from 'h3'
import { agentSessions } from '../../utils/agent-sessions'

interface AbortBody {
  sessionId: string
}

export default defineEventHandler(async (event) => {
  const body = await readBody<AbortBody>(event)
  if (!body?.sessionId) {
    return { ok: false, error: 'Missing sessionId' }
  }

  const session = agentSessions.get(body.sessionId)
  if (session) {
    session.abortController.abort()
    agentSessions.delete(body.sessionId)
  }

  return { ok: true }
})
