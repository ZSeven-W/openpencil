import { defineEventHandler, getQuery, setResponseHeader, setResponseStatus } from 'h3';
import { configureProxyDispatcher } from '../../utils/proxy-dispatcher';

// Route external fetches through the system proxy when set. Same
// rationale as image-search.ts — the user's machine routes outbound
// HTTPS through a local proxy and Node's native fetch ignores it
// without an explicit dispatcher.
configureProxyDispatcher();

/**
 * GET /api/ai/image-proxy?url=<encoded-openverse-thumb-url>
 *
 * Proxies an external image fetch through the dev server so the
 * browser-side image loader doesn't have to reach the upstream host
 * directly. Without this proxy:
 *   - Browser image loader does `img.src = '<openverse-url>'`.
 *   - Browser fetch ignores HTTP_PROXY env vars (only the Node
 *     server-side fetch routes through `EnvHttpProxyAgent`).
 *   - On a machine that requires a proxy to reach openverse.org
 *     (clash / mihomo / corporate gateway), the browser fetch
 *     ECONNREFUSEDs and the canvas shows the placeholder visual
 *     even though the search-pipeline successfully fetched a URL
 *     via the server-side proxy.
 *
 * The image-search endpoint already routes through the proxy. By
 * also routing the IMAGE BYTES through this server endpoint, we
 * guarantee the canvas can paint the photo regardless of the
 * browser's network configuration. The redirect happens at the
 * search-pipeline level — `mapOpenverseResult` rewrites the
 * `thumbUrl` to point at this endpoint.
 *
 * Allow-list: only `https://...` URLs from a small set of known
 * image providers (openverse, wikimedia commons, flickr's static
 * CDN that openverse references). Refusing arbitrary URLs prevents
 * the dev server from being used as an open proxy.
 */
const ALLOWED_HOSTS = new Set([
  'api.openverse.org',
  'commons.wikimedia.org',
  'upload.wikimedia.org',
  'live.staticflickr.com',
  'farm1.staticflickr.com',
  'farm2.staticflickr.com',
  'farm3.staticflickr.com',
  'farm4.staticflickr.com',
  'farm5.staticflickr.com',
  'farm6.staticflickr.com',
  'farm7.staticflickr.com',
  'farm8.staticflickr.com',
  'farm9.staticflickr.com',
]);

export default defineEventHandler(async (event) => {
  const query = getQuery(event);
  const rawUrl = typeof query.url === 'string' ? query.url : '';
  if (!rawUrl) {
    setResponseStatus(event, 400);
    return { error: 'Missing required query param: url' };
  }

  let parsed: URL;
  try {
    parsed = new URL(rawUrl);
  } catch {
    setResponseStatus(event, 400);
    return { error: 'Invalid url' };
  }
  if (parsed.protocol !== 'https:') {
    setResponseStatus(event, 400);
    return { error: 'Only https:// URLs are proxied' };
  }
  if (!ALLOWED_HOSTS.has(parsed.host)) {
    setResponseStatus(event, 403);
    return { error: `Host not in allow-list: ${parsed.host}` };
  }

  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 15000);
    let upstream: Response;
    try {
      upstream = await fetch(parsed.toString(), {
        signal: controller.signal,
        // Some image hosts (openverse thumbs) gate on Accept; an empty
        // Accept makes them happiest.
        headers: { Accept: 'image/*,*/*;q=0.5' },
      });
    } finally {
      clearTimeout(timeoutId);
    }
    if (!upstream.ok) {
      setResponseStatus(event, upstream.status);
      return { error: `Upstream returned ${upstream.status}` };
    }

    // Cap upstream body size. The earlier version did
    // `await upstream.arrayBuffer()` which buffers the entire body
    // in memory with no limit — a malicious or accidentally large
    // upstream (Wikimedia Commons originals can be 100MB+) would
    // happily exhaust the dev server's heap. Cap at 16 MiB (well
    // above any reasonable thumbnail; high-res 4K JPEGs land around
    // 5–8 MiB) and abort the read if we exceed it.
    const declared = upstream.headers.get('content-length');
    if (declared) {
      const declaredBytes = Number.parseInt(declared, 10);
      if (Number.isFinite(declaredBytes) && declaredBytes > MAX_BYTES) {
        controller.abort();
        setResponseStatus(event, 413);
        return {
          error: `Upstream Content-Length ${declaredBytes} exceeds ${MAX_BYTES} cap`,
        };
      }
    }
    if (!upstream.body) {
      setResponseStatus(event, 502);
      return { error: 'Upstream returned no body' };
    }

    // Stream the body and accumulate with a hard cap. Any chunk
    // that pushes total bytes past MAX_BYTES aborts the upstream
    // fetch and returns 413 — no further bytes are buffered.
    const reader = upstream.body.getReader();
    const chunks: Uint8Array[] = [];
    let total = 0;
    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        if (value) {
          total += value.byteLength;
          if (total > MAX_BYTES) {
            controller.abort();
            try {
              await reader.cancel();
            } catch {
              /* ignore cancel errors */
            }
            setResponseStatus(event, 413);
            return {
              error: `Upstream body exceeds ${MAX_BYTES}-byte cap (got ${total}+ so far)`,
            };
          }
          chunks.push(value);
        }
      }
    } finally {
      try {
        reader.releaseLock();
      } catch {
        /* lock already released */
      }
    }

    const contentType = upstream.headers.get('content-type') ?? 'image/jpeg';
    const cacheControl = upstream.headers.get('cache-control') ?? 'public, max-age=86400';

    setResponseHeader(event, 'Content-Type', contentType);
    setResponseHeader(event, 'Cache-Control', cacheControl);
    // Ensure browser canvas fetches don't get blocked by CORS
    // mismatches when the canvas later reads pixels (image-loader
    // sets crossOrigin='anonymous'). Same-origin avoids the issue.
    setResponseHeader(event, 'Access-Control-Allow-Origin', '*');

    return Buffer.concat(chunks.map((c) => Buffer.from(c.buffer, c.byteOffset, c.byteLength)));
  } catch (err) {
    setResponseStatus(event, 502);
    return {
      error: 'Upstream fetch failed',
      detail: err instanceof Error ? err.message : String(err),
    };
  }
});

/**
 * Maximum bytes accepted from any single upstream image. 16 MiB is
 * well above what a thumbnail or even a high-res 4K JPEG needs (~5–
 * 8 MiB). Tuning past this risks a single proxied fetch eating
 * meaningful chunks of the dev server's heap; tuning below it would
 * reject legitimate Wikimedia Commons photos.
 */
const MAX_BYTES = 16 * 1024 * 1024;
