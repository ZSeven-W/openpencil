import { describe, it, expect } from 'vitest'
import { decodeAgentEvent } from '../../src/streaming/sse-decoder'

describe('decodeAgentEvent', () => {
  it('decodes a text event', () => {
    const raw = 'event: text\ndata: {"content":"hello"}\n\n'
    const event = decodeAgentEvent(raw)
    expect(event).toEqual({ type: 'text', content: 'hello' })
  })

  it('decodes a tool_call event', () => {
    const raw = 'event: tool_call\ndata: {"id":"tc_1","name":"read_file","args":{"path":"/foo"},"level":"read"}\n\n'
    const event = decodeAgentEvent(raw)
    expect(event).toEqual({
      type: 'tool_call', id: 'tc_1', name: 'read_file',
      args: { path: '/foo' }, level: 'read',
    })
  })

  it('returns null for malformed input', () => {
    expect(decodeAgentEvent('garbage')).toBeNull()
    expect(decodeAgentEvent('')).toBeNull()
  })
})
