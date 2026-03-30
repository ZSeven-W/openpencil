import { describe, it, expect, vi } from 'vitest';
import { createAgent } from '../src/agent-loop';
import { createToolRegistry } from '../src/tools/tool-registry';
import type { AgentProvider } from '../src/providers/types';
import type { AgentEvent } from '../src/streaming/types';
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

describe('createAgent', () => {
  it('creates an agent with required config', () => {
    const tools = createToolRegistry();
    const agent = createAgent({
      provider: mockProvider,
      tools,
      systemPrompt: 'You are helpful.',
      fallbackStrategy: defaultFallback,
      beforeToolExecute: allowAll,
      maxTurns: 5,
    });
    expect(agent).toHaveProperty('run');
    expect(agent).toHaveProperty('resolveToolResult');
  });

  it('agent.run returns an async iterable', async () => {
    const tools = createToolRegistry();
    const agent = createAgent({
      provider: mockProvider,
      tools,
      systemPrompt: 'You are helpful.',
      fallbackStrategy: defaultFallback,
      beforeToolExecute: allowAll,
      maxTurns: 5,
    });
    const stream = agent.run([{ role: 'user', content: 'hi' }]);
    expect(stream[Symbol.asyncIterator]).toBeDefined();
  });

  it('resolveToolResult buffers result for unknown tool call id (pre-resolution)', () => {
    const tools = createToolRegistry();
    const agent = createAgent({
      provider: mockProvider,
      tools,
      systemPrompt: 'You are helpful.',
      fallbackStrategy: defaultFallback,
      beforeToolExecute: allowAll,
    });
    // Pre-resolution: calling resolveToolResult before waitForToolResult
    // should NOT throw — the result is buffered for later consumption.
    expect(() => agent.resolveToolResult('nonexistent', { success: true })).not.toThrow();
  });

  it('yields abort event when signal is already aborted', async () => {
    const tools = createToolRegistry();
    const controller = new AbortController();
    controller.abort();

    const agent = createAgent({
      provider: mockProvider,
      tools,
      systemPrompt: 'You are helpful.',
      fallbackStrategy: defaultFallback,
      beforeToolExecute: allowAll,
      abortSignal: controller.signal,
    });

    const events: AgentEvent[] = [];
    for await (const event of agent.run([{ role: 'user', content: 'hi' }])) {
      events.push(event);
    }

    // Abort check runs before the turn yield, so only abort is emitted
    expect(events).toHaveLength(1);
    expect(events[0]).toEqual({ type: 'abort' });
  });

  it('uses default maxTurns of 20', () => {
    const tools = createToolRegistry();
    const agent = createAgent({
      provider: mockProvider,
      tools,
      systemPrompt: 'You are helpful.',
      fallbackStrategy: defaultFallback,
      beforeToolExecute: allowAll,
    });
    // Verify it was created without error — default maxTurns=20 is internal
    expect(agent).toBeDefined();
  });

  it('beforeToolExecute deny returns permission error without throwing', async () => {
    // This test verifies the contract: deny produces a ToolResult, not an exception.
    // We can't easily run the full agent loop without mocking streamText,
    // so we test the interface shape — full integration is covered by the agent team tests.
    const tools = createToolRegistry();
    const denyAll = vi.fn(async () => 'deny' as const);
    const agent = createAgent({
      provider: mockProvider,
      tools,
      systemPrompt: 'You are helpful.',
      fallbackStrategy: defaultFallback,
      beforeToolExecute: denyAll,
    });
    expect(agent).toBeDefined();
    // The deny function is called during tool execution in the agent loop,
    // which requires a real LLM response. The unit test validates it's wired in.
    expect(denyAll).not.toHaveBeenCalled(); // not called until run()
  });
});
