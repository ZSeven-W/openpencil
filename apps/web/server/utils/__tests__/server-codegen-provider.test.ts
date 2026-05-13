import { describe, expect, it, vi } from 'vitest';

const agentMocks = vi.hoisted(() => ({
  close: vi.fn(),
  query: vi.fn(),
}));

vi.mock('@anthropic-ai/claude-agent-sdk', () => ({
  query: agentMocks.query,
}));

vi.mock('../resolve-claude-cli', () => ({
  resolveClaudeCli: vi.fn(() => '/usr/local/bin/claude'),
}));

vi.mock('../resolve-claude-agent-env', () => ({
  buildClaudeAgentEnv: vi.fn(() => ({ HOME: '/tmp/openpencil-test' })),
  buildSpawnClaudeCodeProcess: vi.fn(() => undefined),
  getClaudeAgentDebugFilePath: vi.fn(() => undefined),
}));

function claudeResultStream(message: Record<string, unknown>) {
  async function* stream() {
    yield message;
  }
  return Object.assign(stream(), { close: agentMocks.close });
}

describe('server codegen provider', () => {
  it('routes Anthropic background codegen through Claude Agent SDK', async () => {
    const { createChatCompletionText } = await import('../server-codegen-provider');
    agentMocks.close.mockClear();
    agentMocks.query.mockReturnValue(
      claudeResultStream({
        type: 'result',
        subtype: 'success',
        result: 'export default function Design() { return null; }',
      }),
    );

    await expect(
      createChatCompletionText({
        provider: 'anthropic',
        model: 'claude-sonnet-4-6',
        system: 'Generate code.',
        message: 'Build the selected design.',
      }),
    ).resolves.toBe('export default function Design() { return null; }');

    expect(agentMocks.query).toHaveBeenCalledWith(
      expect.objectContaining({
        prompt: 'Build the selected design.',
        options: expect.objectContaining({
          systemPrompt: 'Generate code.',
          model: 'claude-sonnet-4-6',
          maxTurns: 1,
          permissionMode: 'plan',
          persistSession: false,
          pathToClaudeCodeExecutable: '/usr/local/bin/claude',
        }),
      }),
    );
    expect(agentMocks.close).toHaveBeenCalled();
  });

  it('surfaces Anthropic provider failures to the worker', async () => {
    const { createChatCompletionText } = await import('../server-codegen-provider');
    agentMocks.close.mockClear();
    agentMocks.query.mockReturnValue(
      claudeResultStream({
        type: 'result',
        subtype: 'error_max_turns',
        result: 'Claude failed',
        is_error: true,
      }),
    );

    await expect(
      createChatCompletionText({
        provider: 'anthropic',
        model: 'claude-sonnet-4-6',
        system: 'Generate code.',
        message: 'Build the selected design.',
      }),
    ).rejects.toThrow('Claude failed');
    expect(agentMocks.close).toHaveBeenCalled();
  });
});
