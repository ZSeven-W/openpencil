import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { CallOpenAICompatArgs } from '../openai-compat';
import { callOpenAICompat } from '../openai-compat';

type FetchSpy = ReturnType<typeof vi.spyOn<typeof globalThis, 'fetch'>>;
type ConsoleSpy = ReturnType<typeof vi.spyOn<typeof console, 'error'>>;

function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

function textResponse(body: string, status: number): Response {
  return new Response(body, { status });
}

function makeArgs(overrides: Partial<CallOpenAICompatArgs> = {}): CallOpenAICompatArgs {
  return {
    baseURL: 'https://example.test/v1',
    apiKey: 'test-key',
    model: 'test-model',
    system: 'sys',
    user: 'usr',
    label: 'test',
    ...overrides,
  };
}

describe('callOpenAICompat retry behavior', () => {
  let fetchSpy: FetchSpy;
  let consoleErrSpy: ConsoleSpy;

  beforeEach(() => {
    fetchSpy = vi.spyOn(globalThis, 'fetch');
    consoleErrSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);
  });

  afterEach(() => {
    fetchSpy.mockRestore();
    consoleErrSpy.mockRestore();
  });

  it('returns content on first success without retry', async () => {
    fetchSpy.mockResolvedValueOnce(jsonResponse({ choices: [{ message: { content: 'hello' } }] }));
    const out = await callOpenAICompat(makeArgs({ retries: 1 }));
    expect(out).toBe('hello');
    expect(fetchSpy).toHaveBeenCalledTimes(1);
  });

  it('retries once on empty content then succeeds', async () => {
    fetchSpy
      .mockResolvedValueOnce(jsonResponse({ choices: [{ message: { content: '' } }] }))
      .mockResolvedValueOnce(jsonResponse({ choices: [{ message: { content: 'recovered' } }] }));
    const out = await callOpenAICompat(makeArgs({ retries: 1 }));
    expect(out).toBe('recovered');
    expect(fetchSpy).toHaveBeenCalledTimes(2);
  });

  it('retries on HTTP 503 then succeeds', async () => {
    fetchSpy
      .mockResolvedValueOnce(textResponse('upstream', 503))
      .mockResolvedValueOnce(jsonResponse({ choices: [{ message: { content: 'ok' } }] }));
    const out = await callOpenAICompat(makeArgs({ retries: 1 }));
    expect(out).toBe('ok');
    expect(fetchSpy).toHaveBeenCalledTimes(2);
  });

  it('retries on HTTP 429 then succeeds', async () => {
    fetchSpy
      .mockResolvedValueOnce(textResponse('rate limit', 429))
      .mockResolvedValueOnce(jsonResponse({ choices: [{ message: { content: 'ok' } }] }));
    const out = await callOpenAICompat(makeArgs({ retries: 1 }));
    expect(out).toBe('ok');
    expect(fetchSpy).toHaveBeenCalledTimes(2);
  });

  it('does not retry on HTTP 401', async () => {
    fetchSpy.mockResolvedValueOnce(textResponse('bad key', 401));
    await expect(callOpenAICompat(makeArgs({ retries: 1 }))).rejects.toThrow(/HTTP 401/);
    expect(fetchSpy).toHaveBeenCalledTimes(1);
  });

  it('does not retry by default (retries undefined)', async () => {
    fetchSpy.mockResolvedValueOnce(jsonResponse({ choices: [{ message: { content: '' } }] }));
    await expect(callOpenAICompat(makeArgs())).rejects.toThrow(/no content/);
    expect(fetchSpy).toHaveBeenCalledTimes(1);
  });

  it('throws last error after retries exhausted', async () => {
    fetchSpy
      .mockResolvedValueOnce(textResponse('first 503', 503))
      .mockResolvedValueOnce(textResponse('still 503', 503));
    await expect(callOpenAICompat(makeArgs({ retries: 1 }))).rejects.toThrow(/still 503/);
    expect(fetchSpy).toHaveBeenCalledTimes(2);
  });

  it('respects retries=2 (3 attempts total)', async () => {
    fetchSpy
      .mockResolvedValueOnce(jsonResponse({ choices: [{ message: { content: '' } }] }))
      .mockResolvedValueOnce(jsonResponse({ choices: [{ message: { content: '' } }] }))
      .mockResolvedValueOnce(jsonResponse({ choices: [{ message: { content: 'final' } }] }));
    const out = await callOpenAICompat(makeArgs({ retries: 2 }));
    expect(out).toBe('final');
    expect(fetchSpy).toHaveBeenCalledTimes(3);
  });
});
