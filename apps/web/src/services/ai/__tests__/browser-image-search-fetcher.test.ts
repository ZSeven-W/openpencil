import { describe, it, expect, afterEach, vi } from 'vitest';
import { makeBrowserImageSearchFetcher } from '../browser-image-search-fetcher';

/**
 * Browser-side image-search fetcher for inline G() resolution.
 * Tests the happy path + every failure mode the fetcher swallows
 * (returns null rather than throwing).
 */

describe('makeBrowserImageSearchFetcher', () => {
  const originalFetch = global.fetch;

  afterEach(() => {
    global.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  describe('happy path', () => {
    it('POSTs to /api/ai/image-search (relative URL, same-origin)', async () => {
      const spy = vi.fn(
        async () =>
          new Response(JSON.stringify({ results: [{ thumbUrl: 'https://img.example/x.jpg' }] })),
      );
      global.fetch = spy as unknown as typeof fetch;

      const fetcher = makeBrowserImageSearchFetcher();
      const result = await fetcher('sunset');

      expect(result).toBe('https://img.example/x.jpg');
      expect(spy).toHaveBeenCalledOnce();
      const firstCall = spy.mock.calls[0] as unknown as [string, RequestInit];
      expect(firstCall[0]).toBe('/api/ai/image-search');
      expect(firstCall[1].method).toBe('POST');
    });

    it('sends the query in the request body', async () => {
      let body: string | null = null;
      global.fetch = (async (_url, init) => {
        body = (init as RequestInit).body as string;
        return new Response(JSON.stringify({ results: [{ thumbUrl: 'x' }] }));
      }) as typeof fetch;

      const fetcher = makeBrowserImageSearchFetcher();
      await fetcher('mountain lake');

      expect(body).toBeTruthy();
      const parsed = JSON.parse(body!);
      expect(parsed.query).toBe('mountain lake');
      expect(parsed.count).toBe(1);
    });

    it('sets Content-Type: application/json', async () => {
      let headers: Record<string, string> | null = null;
      global.fetch = (async (_url, init) => {
        headers = (init as RequestInit).headers as Record<string, string>;
        return new Response(JSON.stringify({ results: [{ thumbUrl: 'x' }] }));
      }) as typeof fetch;

      const fetcher = makeBrowserImageSearchFetcher();
      await fetcher('x');

      expect(headers!['Content-Type']).toBe('application/json');
    });

    it('returns the FIRST thumbUrl when multiple results', async () => {
      global.fetch = (async () =>
        new Response(
          JSON.stringify({
            results: [
              { thumbUrl: 'first.jpg' },
              { thumbUrl: 'second.jpg' },
              { thumbUrl: 'third.jpg' },
            ],
          }),
        )) as typeof fetch;

      const fetcher = makeBrowserImageSearchFetcher();
      const result = await fetcher('x');
      expect(result).toBe('first.jpg');
    });
  });

  describe('failure modes (all return null, never throw)', () => {
    it('empty query → null without calling fetch', async () => {
      const spy = vi.fn();
      global.fetch = spy as unknown as typeof fetch;

      const fetcher = makeBrowserImageSearchFetcher();
      const result = await fetcher('');
      expect(result).toBeNull();
      expect(spy).not.toHaveBeenCalled();
    });

    it('network error (fetch rejects) → null', async () => {
      global.fetch = (async () => {
        throw new Error('offline');
      }) as typeof fetch;

      const fetcher = makeBrowserImageSearchFetcher();
      const result = await fetcher('x');
      expect(result).toBeNull();
    });

    it('non-ok response (500) → null', async () => {
      global.fetch = (async () => new Response('internal error', { status: 500 })) as typeof fetch;

      const fetcher = makeBrowserImageSearchFetcher();
      const result = await fetcher('x');
      expect(result).toBeNull();
    });

    it('non-JSON body → null', async () => {
      global.fetch = (async () => new Response('<html>not json</html>')) as typeof fetch;

      const fetcher = makeBrowserImageSearchFetcher();
      const result = await fetcher('x');
      expect(result).toBeNull();
    });

    it('missing results field → null', async () => {
      global.fetch = (async () => new Response(JSON.stringify({ message: 'ok' }))) as typeof fetch;

      const fetcher = makeBrowserImageSearchFetcher();
      const result = await fetcher('x');
      expect(result).toBeNull();
    });

    it('empty results array → null', async () => {
      global.fetch = (async () => new Response(JSON.stringify({ results: [] }))) as typeof fetch;

      const fetcher = makeBrowserImageSearchFetcher();
      const result = await fetcher('x');
      expect(result).toBeNull();
    });

    it('first result missing thumbUrl → null', async () => {
      global.fetch = (async () =>
        new Response(JSON.stringify({ results: [{ foo: 'bar' }] }))) as typeof fetch;

      const fetcher = makeBrowserImageSearchFetcher();
      const result = await fetcher('x');
      expect(result).toBeNull();
    });

    it('thumbUrl is empty string → null', async () => {
      global.fetch = (async () =>
        new Response(JSON.stringify({ results: [{ thumbUrl: '' }] }))) as typeof fetch;

      const fetcher = makeBrowserImageSearchFetcher();
      const result = await fetcher('x');
      expect(result).toBeNull();
    });

    it('thumbUrl is a non-string → null', async () => {
      global.fetch = (async () =>
        new Response(JSON.stringify({ results: [{ thumbUrl: 42 }] }))) as typeof fetch;

      const fetcher = makeBrowserImageSearchFetcher();
      const result = await fetcher('x');
      expect(result).toBeNull();
    });

    it('results is not an array → null', async () => {
      global.fetch = (async () =>
        new Response(JSON.stringify({ results: 'not an array' }))) as typeof fetch;

      const fetcher = makeBrowserImageSearchFetcher();
      const result = await fetcher('x');
      expect(result).toBeNull();
    });

    it('data is null → null', async () => {
      global.fetch = (async () => new Response('null')) as typeof fetch;

      const fetcher = makeBrowserImageSearchFetcher();
      const result = await fetcher('x');
      expect(result).toBeNull();
    });
  });

  describe('concurrency / independence', () => {
    it('two fetchers share no state (independent instances)', async () => {
      global.fetch = (async () =>
        new Response(JSON.stringify({ results: [{ thumbUrl: 'x' }] }))) as typeof fetch;

      const f1 = makeBrowserImageSearchFetcher();
      const f2 = makeBrowserImageSearchFetcher();

      const [r1, r2] = await Promise.all([f1('a'), f2('b')]);
      expect(r1).toBe('x');
      expect(r2).toBe('x');
    });
  });
});
