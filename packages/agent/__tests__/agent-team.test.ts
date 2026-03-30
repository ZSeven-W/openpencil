import { describe, it, expect } from 'vitest';
import { createTeam } from '../src/agent-team';
import { createToolRegistry } from '../src/tools/tool-registry';
import type { AgentProvider } from '../src/providers/types';
import type { FallbackStrategy } from '../src/tools/types';

const mockProvider: AgentProvider = {
  id: 'mock',
  maxContextTokens: 100_000,
  supportsThinking: false,
  model: {} as any,
};

const defaultFallback: FallbackStrategy = {
  systemSuffix: '\n\nFallback: output JSON in a code fence.',
  parseResponse: (text: string) => JSON.parse(text),
};

const allowAll = async () => 'allow' as const;

describe('createTeam', () => {
  it('creates a team with lead and members', () => {
    const team = createTeam({
      lead: {
        provider: mockProvider,
        tools: createToolRegistry(),
        systemPrompt: 'You are a lead.',
      },
      members: [
        {
          id: 'worker',
          provider: mockProvider,
          tools: createToolRegistry(),
          systemPrompt: 'You are a worker.',
        },
      ],
      fallbackStrategy: defaultFallback,
      beforeToolExecute: allowAll,
    });
    expect(team).toHaveProperty('run');
    expect(team).toHaveProperty('abort');
  });

  it('accepts different providers for lead and members', () => {
    const otherProvider: AgentProvider = { ...mockProvider, id: 'other' };
    const team = createTeam({
      lead: { provider: mockProvider, tools: createToolRegistry(), systemPrompt: 'Lead' },
      members: [
        {
          id: 'member1',
          provider: otherProvider,
          tools: createToolRegistry(),
          systemPrompt: 'Member',
        },
      ],
      fallbackStrategy: defaultFallback,
      beforeToolExecute: allowAll,
    });
    expect(team).toBeDefined();
  });

  it('does not mutate the caller tools registry', () => {
    const tools = createToolRegistry();
    const initialCount = tools.list().length;
    createTeam({
      lead: { provider: mockProvider, tools, systemPrompt: 'Lead' },
      members: [
        {
          id: 'worker',
          provider: mockProvider,
          tools: createToolRegistry(),
          systemPrompt: 'Worker',
        },
      ],
      fallbackStrategy: defaultFallback,
      beforeToolExecute: allowAll,
    });
    expect(tools.list().length).toBe(initialCount);
  });
});
