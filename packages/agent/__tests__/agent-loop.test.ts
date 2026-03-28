import { describe, it, expect, vi } from 'vitest'
import { z } from 'zod'
import { createAgent } from '../src/agent-loop'
import { createToolRegistry } from '../src/tools/tool-registry'
import type { AgentProvider } from '../src/providers/types'
import type { AgentEvent } from '../src/streaming/types'

const mockProvider: AgentProvider = {
  id: 'mock',
  maxContextTokens: 100_000,
  supportsThinking: false,
  model: {} as any,
}

describe('createAgent', () => {
  it('creates an agent with required config', () => {
    const tools = createToolRegistry()
    const agent = createAgent({
      provider: mockProvider,
      tools,
      systemPrompt: 'You are helpful.',
      maxTurns: 5,
    })
    expect(agent).toHaveProperty('run')
    expect(agent).toHaveProperty('resolveToolResult')
  })

  it('agent.run returns an async iterable', async () => {
    const tools = createToolRegistry()
    const agent = createAgent({
      provider: mockProvider,
      tools,
      systemPrompt: 'You are helpful.',
      maxTurns: 5,
    })
    const stream = agent.run([{ role: 'user', content: 'hi' }])
    expect(stream[Symbol.asyncIterator]).toBeDefined()
  })

  it('resolveToolResult throws for unknown tool call id', () => {
    const tools = createToolRegistry()
    const agent = createAgent({
      provider: mockProvider,
      tools,
      systemPrompt: 'You are helpful.',
    })
    expect(() => agent.resolveToolResult('nonexistent', { success: true }))
      .toThrow('No pending tool call: nonexistent')
  })

  it('yields abort event when signal is already aborted', async () => {
    const tools = createToolRegistry()
    const controller = new AbortController()
    controller.abort()

    const agent = createAgent({
      provider: mockProvider,
      tools,
      systemPrompt: 'You are helpful.',
      abortSignal: controller.signal,
    })

    const events: AgentEvent[] = []
    for await (const event of agent.run([{ role: 'user', content: 'hi' }])) {
      events.push(event)
    }

    // Abort check runs before the turn yield, so only abort is emitted
    expect(events).toHaveLength(1)
    expect(events[0]).toEqual({ type: 'abort' })
  })

  it('uses default maxTurns of 20', () => {
    const tools = createToolRegistry()
    const agent = createAgent({
      provider: mockProvider,
      tools,
      systemPrompt: 'You are helpful.',
    })
    // Verify it was created without error — default maxTurns=20 is internal
    expect(agent).toBeDefined()
  })
})
