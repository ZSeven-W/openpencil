import { describe, expect, it } from 'vitest';
import {
  buildProviderModelsURL,
  normalizeBaseURL,
  normalizeMemberBaseURL,
  normalizeOptionalBaseURL,
  requireOpenAICompatBaseURL,
} from '../api/ai/provider-url';

describe('provider-url helpers', () => {
  it('normalizes whitespace and trailing slashes', () => {
    expect(normalizeBaseURL(' https://api.openai.com/v1/ ')).toBe('https://api.openai.com/v1');
    expect(normalizeBaseURL('https://openrouter.ai/api/v1///')).toBe(
      'https://openrouter.ai/api/v1',
    );
  });

  it('normalizes optional baseURL to undefined when empty', () => {
    expect(normalizeOptionalBaseURL(undefined)).toBeUndefined();
    expect(normalizeOptionalBaseURL('   ')).toBeUndefined();
  });

  it('builds /models URL from canonical API root baseURL', () => {
    expect(buildProviderModelsURL('https://api.openai.com/v1/')).toBe(
      'https://api.openai.com/v1/models',
    );
    expect(buildProviderModelsURL('https://generativelanguage.googleapis.com/v1beta/openai')).toBe(
      'https://generativelanguage.googleapis.com/v1beta/openai/models',
    );
    expect(buildProviderModelsURL('https://ark.cn-beijing.volces.com/api/v3')).toBe(
      'https://ark.cn-beijing.volces.com/api/v3/models',
    );
  });

  it('requires baseURL for openai-compatible providers', () => {
    expect(() => requireOpenAICompatBaseURL(undefined)).toThrow(
      'OpenAI-compatible provider requires baseURL',
    );
    expect(() => requireOpenAICompatBaseURL('   ')).toThrow(
      'OpenAI-compatible provider requires baseURL',
    );
    expect(requireOpenAICompatBaseURL('https://api.openai.com/v1/')).toBe(
      'https://api.openai.com/v1',
    );
  });

  it('validates team-member baseURL for openai-compat', () => {
    expect(() => normalizeMemberBaseURL('designer', 'openai-compat', undefined)).toThrow(
      'Member "designer" (openai-compat) requires baseURL',
    );
    expect(() => normalizeMemberBaseURL('designer', 'openai-compat', '   ')).toThrow(
      'Member "designer" (openai-compat) requires baseURL',
    );
    expect(normalizeMemberBaseURL('designer', 'openai-compat', 'https://api.openai.com/v1/')).toBe(
      'https://api.openai.com/v1',
    );
  });

  it('allows missing baseURL for anthropic team members', () => {
    expect(normalizeMemberBaseURL('lead', 'anthropic', undefined)).toBeUndefined();
    expect(normalizeMemberBaseURL('lead', 'anthropic', 'https://custom.api.com/')).toBe(
      'https://custom.api.com',
    );
  });
});
