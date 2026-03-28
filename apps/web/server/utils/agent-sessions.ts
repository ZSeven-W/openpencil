import type { Agent } from '@zseven-w/agent'

export interface AgentSession {
  agent: Agent
  abortController: AbortController
  createdAt: number
  lastActivity: number
}

export const agentSessions = new Map<string, AgentSession>()

// Cleanup stale sessions every 60s (5-minute TTL from last activity)
setInterval(() => {
  try {
    const now = Date.now()
    for (const [id, session] of agentSessions) {
      if (now - session.lastActivity > 5 * 60 * 1000) {
        try { session.abortController.abort() } catch { /* ignore */ }
        agentSessions.delete(id)
      }
    }
  } catch { /* ignore cleanup errors */ }
}, 60_000)
