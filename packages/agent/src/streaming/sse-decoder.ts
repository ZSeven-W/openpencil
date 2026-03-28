import type { AgentEvent } from './types'

export function decodeAgentEvent(raw: string): AgentEvent | null {
  const eventMatch = raw.match(/^event:\s*(\S+)/)
  const dataMatch = raw.match(/^data:\s*(.+)$/m)
  if (!eventMatch || !dataMatch) return null
  try {
    const type = eventMatch[1] as AgentEvent['type']
    const payload = JSON.parse(dataMatch[1])
    return { type, ...payload } as AgentEvent
  } catch {
    return null
  }
}
