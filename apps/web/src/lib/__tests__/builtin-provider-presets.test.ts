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
