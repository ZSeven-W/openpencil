/**
 * Verify that every openai-compat preset's baseURL produces
 * the correct chat completions endpoint.
 *
 * The Zig agent-native module constructs: `${baseURL}/chat/completions`
 * So each preset's baseURL must be the API root WITHOUT a trailing /v1
 * for providers that don't use one (e.g. Ark, Zhipu), or WITH /v1 for
 * providers that require it (e.g. OpenAI, DeepSeek).
 */
import { describe, expect, it } from 'vitest';
import { BUILTIN_PROVIDER_PRESETS } from '../../src/lib/builtin-provider-presets';
import { requireOpenAICompatBaseURL } from '../api/ai/provider-url';

/** Known correct chat completions endpoints per provider */
const EXPECTED_ENDPOINTS: Record<string, string> = {
  openai: 'https://api.openai.com/v1/chat/completions',
  openrouter: 'https://openrouter.ai/api/v1/chat/completions',
  deepseek: 'https://api.deepseek.com/v1/chat/completions',
  gemini: 'https://generativelanguage.googleapis.com/v1beta/openai/chat/completions',
  zhipu: 'https://open.bigmodel.cn/api/paas/v4/chat/completions',
  kimi: 'https://api.moonshot.cn/v1/chat/completions',
  bailian: 'https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions',
  doubao: 'https://ark.cn-beijing.volces.com/api/v3/chat/completions',
  'ark-coding': 'https://ark.cn-beijing.volces.com/api/coding/v3/chat/completions',
  xiaomi: 'https://api.xiaomimimo.com/v1/chat/completions',
  modelscope: 'https://api-inference.modelscope.cn/v1/chat/completions',
  stepfun: 'https://api.stepfun.com/v1/chat/completions',
  nvidia: 'https://integrate.api.nvidia.com/v1/chat/completions',
};

describe('preset baseURL → chat/completions endpoint', () => {
  for (const [presetId, expectedURL] of Object.entries(EXPECTED_ENDPOINTS)) {
    it(`${presetId}: ${expectedURL}`, () => {
      const preset = BUILTIN_PROVIDER_PRESETS[presetId as keyof typeof BUILTIN_PROVIDER_PRESETS];
      expect(preset).toBeDefined();
      expect(preset.type).toBe('openai-compat');

      // Simulate server-side normalization
      const normalized = requireOpenAICompatBaseURL(preset.baseURL);
      // Simulate Zig-side URL construction
      const finalURL = `${normalized}/chat/completions`;

      expect(finalURL).toBe(expectedURL);
    });
  }

  it('all openai-compat presets have a baseURL or regions', () => {
    for (const [id, preset] of Object.entries(BUILTIN_PROVIDER_PRESETS)) {
      if (preset.type === 'openai-compat' && id !== 'custom') {
        expect(
          preset.baseURL || preset.regions,
          `preset "${id}" missing both baseURL and regions`,
        ).toBeTruthy();
      }
    }
  });

  it('glm-coding regions produce correct endpoints', () => {
    const preset = BUILTIN_PROVIDER_PRESETS['glm-coding'];
    expect(preset.regions).toBeDefined();

    const cn = requireOpenAICompatBaseURL(preset.regions!.cn.baseURL);
    expect(`${cn}/chat/completions`).toBe(
      'https://open.bigmodel.cn/api/coding/paas/v4/chat/completions',
    );

    const global = requireOpenAICompatBaseURL(preset.regions!.global.baseURL);
    expect(`${global}/chat/completions`).toBe(
      'https://api.z.ai/api/coding/paas/v4/chat/completions',
    );
  });
});
