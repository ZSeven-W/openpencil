import { describe, it, expect } from 'vitest'
import { createTeam } from '../src/agent-team'
import { createToolRegistry } from '../src/tools/tool-registry'
import type { AgentProvider } from '../src/providers/types'

const mockProvider: AgentProvider = {
  id: 'mock',
  maxContextTokens: 100_000,
  supportsThinking: false,
  model: {} as any,
}

describe('createTeam', () => {
  it('creates a team with lead and members', () => {
    const team = createTeam({
      lead: {
        provider: mockProvider,
        tools: createToolRegistry(),
        systemPrompt: 'You are a lead.',
      },
      members: [
        { id: 'worker', provider: mockProvider, tools: createToolRegistry(), systemPrompt: 'You are a worker.' },
      ],
    })
    expect(team).toHaveProperty('run')
    expect(team).toHaveProperty('abort')
  })

  it('accepts different providers for lead and members', () => {
    const otherProvider: AgentProvider = { ...mockProvider, id: 'other' }
    const team = createTeam({
      lead: { provider: mockProvider, tools: createToolRegistry(), systemPrompt: 'Lead' },
      members: [
        { id: 'member1', provider: otherProvider, tools: createToolRegistry(), systemPrompt: 'Member' },
      ],
    })
    expect(team).toBeDefined()
  })

  it('does not mutate the caller tools registry', () => {
    const tools = createToolRegistry()
    const initialCount = tools.list().length
    createTeam({
      lead: { provider: mockProvider, tools, systemPrompt: 'Lead' },
      members: [{ id: 'worker', provider: mockProvider, tools: createToolRegistry(), systemPrompt: 'Worker' }],
    })
    expect(tools.list().length).toBe(initialCount)
  })
})
