import type { AgentEvent } from './types'

export function encodeAgentEvent(event: AgentEvent): string {
  const { type, ...payload } = event
  return `event: ${type}\ndata: ${JSON.stringify(payload)}\n\n`
}
