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

  it('encodes a text event with source', () => {
    const event: AgentEvent = { type: 'text', content: 'hello', source: 'lead' }
    const encoded = encodeAgentEvent(event)
    expect(encoded).toBe('event: text\ndata: {"content":"hello","source":"lead"}\n\n')
  })

  it('encodes a member_start event', () => {
    const event: AgentEvent = { type: 'member_start', memberId: 'designer', task: 'Create landing page' }
    const encoded = encodeAgentEvent(event)
    expect(encoded).toBe('event: member_start\ndata: {"memberId":"designer","task":"Create landing page"}\n\n')
  })

  it('encodes a member_end event', () => {
    const event: AgentEvent = { type: 'member_end', memberId: 'designer', result: 'Done' }
    const encoded = encodeAgentEvent(event)
    expect(encoded).toBe('event: member_end\ndata: {"memberId":"designer","result":"Done"}\n\n')
  })
})
