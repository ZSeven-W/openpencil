import { describe, it, expect } from 'vitest'
import { encodeAgentEvent } from '../../src/streaming/sse-encoder'
import type { AgentEvent } from '../../src/streaming/types'

describe('encodeAgentEvent', () => {
  it('encodes a text event', () => {
    const event: AgentEvent = { type: 'text', content: 'hello' }
    const encoded = encodeAgentEvent(event)
    expect(encoded).toBe('event: text\ndata: {"content":"hello"}\n\n')
  })

  it('encodes a tool_call event', () => {
    const event: AgentEvent = {
      type: 'tool_call', id: 'tc_1', name: 'read_file',
      args: { path: '/foo' }, level: 'read',
    }
    const encoded = encodeAgentEvent(event)
    expect(encoded).toContain('event: tool_call')
    expect(encoded).toContain('"id":"tc_1"')
  })

  it('encodes a done event', () => {
    const event: AgentEvent = { type: 'done', totalTurns: 3 }
    const encoded = encodeAgentEvent(event)
    expect(encoded).toBe('event: done\ndata: {"totalTurns":3}\n\n')
  })
})
