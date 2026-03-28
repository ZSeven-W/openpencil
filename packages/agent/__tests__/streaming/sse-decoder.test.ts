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

  it('decodes a text event with source', () => {
    const raw = 'event: text\ndata: {"content":"hello","source":"designer"}'
    const event = decodeAgentEvent(raw)
    expect(event).toEqual({ type: 'text', content: 'hello', source: 'designer' })
  })

  it('decodes a member_start event', () => {
    const raw = 'event: member_start\ndata: {"memberId":"designer","task":"Build UI"}'
    const event = decodeAgentEvent(raw)
    expect(event).toEqual({ type: 'member_start', memberId: 'designer', task: 'Build UI' })
  })

  it('decodes a member_end event', () => {
    const raw = 'event: member_end\ndata: {"memberId":"designer","result":"Done"}'
    const event = decodeAgentEvent(raw)
    expect(event).toEqual({ type: 'member_end', memberId: 'designer', result: 'Done' })
  })
})
