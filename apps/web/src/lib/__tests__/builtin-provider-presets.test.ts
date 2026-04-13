import { describe, expect, it } from 'vitest';
import {
  BUILTIN_PROVIDER_PRESETS,
  canonicalizeBuiltinProviderConfig,
  inferBuiltinProviderPreset,
  inferBuiltinProviderRegion,
} from '../builtin-provider-presets';

describe('builtin provider presets', () => {
  it('infers all known non-custom presets from their canonical base URLs', () => {
    for (const [preset, cfg] of Object.entries(BUILTIN_PROVIDER_PRESETS)) {
      if (preset === 'custom') continue;

      if (cfg.baseURL) {
        expect(
          inferBuiltinProviderPreset({
            type: cfg.type,
            baseURL: cfg.baseURL,
          } as any),
        ).toBe(preset);
      }

      if (cfg.regions) {
        expect(
          inferBuiltinProviderPreset({
            type: cfg.type,
            baseURL: cfg.regions.cn.baseURL,
          } as any),
        ).toBe(preset);
        expect(
          inferBuiltinProviderPreset({
            type: cfg.type,
            baseURL: cfg.regions.global.baseURL,
          } as any),
        ).toBe(preset);
      }
    }
  });

  it('keeps MiniMax distinct from Anthropic when inferring preset and region', () => {
    expect(
      inferBuiltinProviderPreset({
        type: 'anthropic',
        baseURL: 'https://api.minimaxi.com/anthropic',
      } as any),
    ).toBe('minimax');
    expect(
      inferBuiltinProviderRegion({
        type: 'anthropic',
        baseURL: 'https://api.minimax.io/anthropic',
      } as any),
    ).toBe('global');
  });

  it('keeps coding-plan endpoints distinct from standard zhipu and doubao endpoints', () => {
    expect(
      inferBuiltinProviderPreset({
        type: 'openai-compat',
        baseURL: 'https://open.bigmodel.cn/api/coding/paas/v4',
      } as any),
    ).toBe('glm-coding');
    expect(
      inferBuiltinProviderRegion({
        type: 'openai-compat',
        baseURL: 'https://api.z.ai/api/coding/paas/v4',
      } as any),
    ).toBe('global');
    expect(
      inferBuiltinProviderPreset({
        type: 'openai-compat',
        baseURL: 'https://open.bigmodel.cn/api/paas/v4',
      } as any),
    ).toBe('zhipu');
    expect(
      inferBuiltinProviderPreset({
        type: 'openai-compat',
        baseURL: 'https://ark.cn-beijing.volces.com/api/coding/v3',
      } as any),
    ).toBe('ark-coding');
    expect(
      inferBuiltinProviderPreset({
        type: 'openai-compat',
        baseURL: 'https://ark.cn-beijing.volces.com/api/v3',
      } as any),
    ).toBe('doubao');
  });

  it('migrates legacy Z.AI global endpoints to the canonical hostnames', () => {
    const general = canonicalizeBuiltinProviderConfig({
      id: 'bp-zhipu-global',
      displayName: 'Zhipu',
      type: 'openai-compat',
      apiKey: 'key',
      model: 'glm-5',
      baseURL: 'https://open.z.ai/api/paas/v4',
      enabled: true,
    });

    expect(general.preset).toBe('zhipu');
    expect(general.baseURL).toBe('https://api.z.ai/api/paas/v4');

    const coding = canonicalizeBuiltinProviderConfig({
      id: 'bp-glm-coding-global',
      displayName: 'GLM Coding Plan',
      type: 'openai-compat',
      apiKey: 'key',
      model: 'glm-4.7',
      baseURL: 'https://open.z.ai/api/coding/paas/v4',
      enabled: true,
    });

    expect(coding.preset).toBe('glm-coding');
    expect(coding.baseURL).toBe('https://api.z.ai/api/coding/paas/v4');
  });

  it('migrates malformed Ark URLs with an extra /v1 suffix back to canonical roots', () => {
    const doubao = canonicalizeBuiltinProviderConfig({
      id: 'bp-doubao-bad',
      displayName: 'DouBao',
      type: 'openai-compat',
      apiKey: 'key',
      model: 'doubao-seed-2.0-code',
      baseURL: 'https://ark.cn-beijing.volces.com/api/v3/v1',
      enabled: true,
    });

    expect(doubao.preset).toBe('doubao');
    expect(doubao.baseURL).toBe('https://ark.cn-beijing.volces.com/api/v3');

    const arkCoding = canonicalizeBuiltinProviderConfig({
      id: 'bp-ark-coding-bad',
      displayName: 'Ark Coding Plan',
      type: 'openai-compat',
      apiKey: 'key',
      model: 'ark-code-latest',
      baseURL: 'https://ark.cn-beijing.volces.com/api/coding/v3/v1',
      enabled: true,
    });

    expect(arkCoding.preset).toBe('ark-coding');
    expect(arkCoding.baseURL).toBe('https://ark.cn-beijing.volces.com/api/coding/v3');
  });

  it('keeps custom preset URLs unchanged (version detection is in agent-native)', () => {
    const custom = canonicalizeBuiltinProviderConfig({
      id: 'bp-custom-ark',
      displayName: 'Custom Ark',
      preset: 'custom',
      type: 'openai-compat',
      apiKey: 'key',
      model: 'ark-code-latest',
      baseURL: 'https://ark.cn-beijing.volces.com/api/coding/v3',
      enabled: true,
    });

    expect(custom.preset).toBe('custom');
    expect(custom.baseURL).toBe('https://ark.cn-beijing.volces.com/api/coding/v3');
  });

  it('canonicalizes legacy built-in provider URLs on hydrate', () => {
    expect(
      canonicalizeBuiltinProviderConfig({
        id: 'bp-openai',
        displayName: 'OpenAI',
        type: 'openai-compat',
        apiKey: 'sk-test',
        model: 'gpt-5.4',
        preset: 'openai',
        baseURL: 'https://api.openai.com',
        enabled: true,
      }).baseURL,
    ).toBe('https://api.openai.com/v1');

    expect(
      canonicalizeBuiltinProviderConfig({
        id: 'bp-minimax',
        displayName: 'MiniMax',
        type: 'anthropic',
        apiKey: 'key',
        model: 'MiniMax-M2.7',
        baseURL: 'https://api.minimaxi.com/anthropic/v1',
        enabled: true,
      }).baseURL,
    ).toBe('https://api.minimaxi.com/anthropic');
  });

  it('preserves alternative-format baseURL on canonicalize (e.g. Bailian Coding Plan + Anthropic)', () => {
    // Repro for the regression where switching a default-OpenAI preset to
    // its Anthropic alt URL would silently get reset to the OpenAI URL on
    // save, and the request then went to .../v1/messages → 404.
    const altFormat = canonicalizeBuiltinProviderConfig({
      id: 'bp-bailian-coding-anthropic',
      displayName: 'Bailian Coding Plan',
      preset: 'bailian-coding',
      type: 'anthropic',
      apiKey: 'sk-test',
      model: 'qwen3-coder-plus',
      baseURL: 'https://coding.dashscope.aliyuncs.com/apps/anthropic',
      enabled: true,
    });

    expect(altFormat.preset).toBe('bailian-coding');
    expect(altFormat.type).toBe('anthropic');
    expect(altFormat.baseURL).toBe('https://coding.dashscope.aliyuncs.com/apps/anthropic');
  });

  it('round-trips the default OpenAI URL on canonicalize for the same preset', () => {
    // Sanity: we did not break the default-format path while fixing the alt-format one.
    const defaultFormat = canonicalizeBuiltinProviderConfig({
      id: 'bp-bailian-coding-openai',
      displayName: 'Bailian Coding Plan',
      preset: 'bailian-coding',
      type: 'openai-compat',
      apiKey: 'sk-test',
      model: 'qwen3-coder-plus',
      baseURL: 'https://coding.dashscope.aliyuncs.com/v1',
      enabled: true,
    });

    expect(defaultFormat.preset).toBe('bailian-coding');
    expect(defaultFormat.type).toBe('openai-compat');
    expect(defaultFormat.baseURL).toBe('https://coding.dashscope.aliyuncs.com/v1');
  });

  it('infers a unique preset from its alternative-format URL (so reload restores it correctly)', () => {
    // Previously the URL→preset reverse lookup only knew about default-format
    // URLs; an alt-format URL would fall through to 'custom' and lose the
    // preset selection on the next reload.
    expect(
      inferBuiltinProviderPreset({
        type: 'anthropic',
        baseURL: 'https://coding.dashscope.aliyuncs.com/apps/anthropic',
      } as any),
    ).toBe('bailian-coding');
  });

  it('preserves an explicit preset when two presets share the same alt URL', () => {
    // zhipu and glm-coding both point at https://open.bigmodel.cn/api/anthropic
    // for their Anthropic alt format. The user's dropdown choice must win.
    const glmCoding = canonicalizeBuiltinProviderConfig({
      id: 'bp-glm-coding-anthropic',
      displayName: 'GLM Coding Plan',
      preset: 'glm-coding',
      type: 'anthropic',
      apiKey: 'key',
      model: 'glm-4.7',
      baseURL: 'https://open.bigmodel.cn/api/anthropic',
      enabled: true,
    });
    expect(glmCoding.preset).toBe('glm-coding');
    expect(glmCoding.baseURL).toBe('https://open.bigmodel.cn/api/anthropic');

    const zhipu = canonicalizeBuiltinProviderConfig({
      id: 'bp-zhipu-anthropic',
      displayName: 'Zhipu',
      preset: 'zhipu',
      type: 'anthropic',
      apiKey: 'key',
      model: 'glm-5',
      baseURL: 'https://open.bigmodel.cn/api/anthropic',
      enabled: true,
    });
    expect(zhipu.preset).toBe('zhipu');
    expect(zhipu.baseURL).toBe('https://open.bigmodel.cn/api/anthropic');
  });

  it('detects global region from an alt-format URL', () => {
    expect(
      inferBuiltinProviderRegion({
        type: 'anthropic',
        baseURL: 'https://coding-intl.dashscope.aliyuncs.com/apps/anthropic',
      } as any),
    ).toBe('global');
  });

  it('prefers a recognized legacy URL over stale built-in preset metadata during migration', () => {
    const migrated = canonicalizeBuiltinProviderConfig({
      id: 'bp-stale',
      displayName: 'MiniMax',
      type: 'anthropic',
      apiKey: 'key',
      model: 'MiniMax-M2.7',
      preset: 'anthropic',
      baseURL: 'https://api.minimax.io/anthropic/v1',
      enabled: true,
    });

    expect(migrated.preset).toBe('minimax');
    expect(migrated.baseURL).toBe('https://api.minimax.io/anthropic');
  });
});
