import type { ImageSearchFetcher } from '@zseven-w/pen-mcp';

/**
 * Browser-side image-search fetcher for inline `G()` resolution
 * in the batch_design DSL.
 *
 * The default browser dispatch path (`element-tools-dispatcher.ts::
 * applyBatchDesignDsl`) INTENTIONALLY omits this fetcher — G() ops
 * leave `src` empty and the downstream `scanAndFillImages` pass
 * enriches them asynchronously. That avoids per-G() network round-
 * trips during DSL execution, which can blow up latency when a
 * batch contains 10+ image ops.
 *
 * This helper is provided for callers that WANT inline resolution
 * (for example: composition smoke tests that need a deterministic
 * src, or a user-opt-in preview mode). It uses the relative URL
 * `/api/ai/image-search` — same-origin fetch, so no CORS
 * considerations and no dependency on `getSyncUrl()` the way the
 * server-side fetcher has.
 *
 * Shape matches the API:
 *   POST /api/ai/image-search
 *   Body: {"query": "...", "count": 1}
 *   Response: {"results": [{"thumbUrl": "..."}]}
 *
 * Returns the first `thumbUrl` or null on any failure (empty
 * results, network error, non-JSON body, malformed shape). NEVER
 * throws — callers can slot it into the DSL executor without
 * try/catch wrappers.
 */
export function makeBrowserImageSearchFetcher(): ImageSearchFetcher {
  return async (prompt: string): Promise<string | null> => {
    if (typeof prompt !== 'string' || prompt.length === 0) return null;
    try {
      const res = await fetch('/api/ai/image-search', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ query: prompt, count: 1 }),
      });
      if (!res.ok) return null;
      const data = (await res.json()) as unknown;
      if (!data || typeof data !== 'object') return null;
      const results = (data as { results?: unknown }).results;
      if (!Array.isArray(results) || results.length === 0) return null;
      const first = results[0];
      if (!first || typeof first !== 'object') return null;
      const thumbUrl = (first as { thumbUrl?: unknown }).thumbUrl;
      if (typeof thumbUrl !== 'string' || thumbUrl.length === 0) return null;
      return thumbUrl;
    } catch {
      return null;
    }
  };
}
