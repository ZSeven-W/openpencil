import { describe, expect, it } from 'vitest';
import {
  buildProviderModelsURL,
  formatFetchError,
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

describe('formatFetchError', () => {
  it('unwraps undici "fetch failed" by reading error.cause', () => {
    const cause = Object.assign(new Error('Client network socket disconnected'), {
      code: 'ECONNRESET',
    });
    const err = Object.assign(new TypeError('fetch failed'), { cause });
    expect(formatFetchError(err)).toBe('ECONNRESET: Client network socket disconnected');
  });

  it('skips the prefix when the cause message already contains the code', () => {
    const cause = Object.assign(new Error('getaddrinfo ENOTFOUND api.example.com'), {
      code: 'ENOTFOUND',
    });
    const err = Object.assign(new TypeError('fetch failed'), { cause });
    expect(formatFetchError(err)).toBe('getaddrinfo ENOTFOUND api.example.com');
  });

  it('returns just the cause code when message is missing', () => {
    const cause = Object.assign(new Error(''), { code: 'ECONNREFUSED' });
    const err = Object.assign(new TypeError('fetch failed'), { cause });
    expect(formatFetchError(err)).toBe('ECONNREFUSED');
  });

  it('returns just the cause message when no code', () => {
    const cause = new Error('self-signed certificate in certificate chain');
    const err = Object.assign(new TypeError('fetch failed'), { cause });
    expect(formatFetchError(err)).toBe('self-signed certificate in certificate chain');
  });

  it('falls back to error.message when no cause', () => {
    expect(formatFetchError(new Error('Provider returned 401'))).toBe('Provider returned 401');
  });

  it('handles non-Error inputs', () => {
    expect(formatFetchError('boom')).toBe('Unknown error');
    expect(formatFetchError(undefined)).toBe('Unknown error');
  });

  it('walks nested cause chains', () => {
    const root = Object.assign(new Error('Hostname does not match certificate'), {
      code: 'ERR_TLS_CERT_ALTNAME_INVALID',
    });
    const wrapper = Object.assign(new Error('TLS handshake failed'), { cause: root });
    const outer = Object.assign(new TypeError('fetch failed'), { cause: wrapper });
    expect(formatFetchError(outer)).toBe(
      'ERR_TLS_CERT_ALTNAME_INVALID: Hostname does not match certificate',
    );
  });

  it('unwraps AggregateError so per-IP attempt reasons reach the user', () => {
    const a = Object.assign(new Error('connect ECONNREFUSED 127.0.0.1:8080'), {
      code: 'ECONNREFUSED',
    });
    const b = Object.assign(new Error('connect ECONNREFUSED ::1:8080'), { code: 'ECONNREFUSED' });
    const agg = Object.assign(new AggregateError([a, b], 'all attempts failed'));
    const err = Object.assign(new TypeError('fetch failed'), { cause: agg });
    expect(formatFetchError(err)).toBe(
      'connect ECONNREFUSED 127.0.0.1:8080; connect ECONNREFUSED ::1:8080',
    );
  });
});
