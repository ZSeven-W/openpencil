import { describe, expect, it } from 'vitest';
import { resolveCodegenModelConfig } from './codegen-model-config';

describe('resolveCodegenModelConfig', () => {
  it('resolves a connected model from model groups', () => {
    expect(
      resolveCodegenModelConfig(
        'claude-sonnet',
        [
          {
            provider: 'anthropic',
            providerName: 'Anthropic',
            models: [
              {
                value: 'claude-sonnet',
                displayName: 'Sonnet',
                description: '',
                provider: 'anthropic',
              },
            ],
          },
        ],
        [],
      ),
    ).toMatchObject({ model: 'claude-sonnet', provider: 'anthropic' });
  });

  it('returns null for a model that has no connected provider', () => {
    expect(resolveCodegenModelConfig('claude-sonnet', [], [])).toBeNull();
  });

  it('routes enabled builtin providers through the builtin server path', () => {
    const builtinProvider = {
      id: 'bp-1',
      displayName: 'Custom',
      type: 'openai-compat' as const,
      apiKey: 'key',
      model: 'model-a',
      enabled: true,
    };

    expect(
      resolveCodegenModelConfig(
        'builtin:bp-1:model-a',
        [{ provider: 'openai', providerName: 'Custom', models: [] }],
        [builtinProvider],
      ),
    ).toEqual({
      model: 'builtin:bp-1:model-a',
      provider: 'builtin',
      builtinProvider,
    });
  });

  it('rejects disabled builtin providers before starting a cloud job', () => {
    expect(
      resolveCodegenModelConfig('builtin:bp-1:model-a', [], [
        {
          id: 'bp-1',
          displayName: 'Custom',
          type: 'openai-compat',
          apiKey: 'key',
          model: 'model-a',
          enabled: false,
        },
      ]),
    ).toBeNull();
  });
});
