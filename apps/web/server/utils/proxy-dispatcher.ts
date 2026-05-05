/**
 * Configure undici's global fetch dispatcher to honor `HTTPS_PROXY` /
 * `HTTP_PROXY` env vars. Node's native `fetch` (built on undici) does
 * NOT auto-route through the system proxy the way `curl` does — it
 * goes direct, which fails with `ECONNREFUSED` on machines that route
 * outbound HTTPS through a local proxy (clash / mihomo / corporate
 * gateways). The image-search endpoint already swallows that failure
 * and silently falls back to Wikimedia, but Wikimedia is also blocked
 * on the same machines, so designs land with empty image placeholders.
 *
 * Calling `setGlobalDispatcher(new ProxyAgent(...))` once at module
 * load makes every `fetch()` in the server route through the proxy.
 * When no proxy env var is set (production deploys, CI) the function
 * is a no-op — the default global dispatcher continues to handle
 * direct connections.
 *
 * Idempotent: subsequent calls return without re-installing the
 * dispatcher, so callers can safely import and invoke from any
 * endpoint that makes external fetches.
 */
let configured = false;

export function configureProxyDispatcher(): void {
  if (configured) return;
  configured = true;

  const proxy =
    process.env.HTTPS_PROXY ??
    process.env.https_proxy ??
    process.env.HTTP_PROXY ??
    process.env.http_proxy;
  if (!proxy) return;

  try {
    // Dynamic require so production builds that strip undici (or run
    // on non-node runtimes) don't crash at import time.
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    const undici = require('undici') as {
      setGlobalDispatcher: (d: unknown) => void;
      ProxyAgent: new (uri: string) => unknown;
    };
    undici.setGlobalDispatcher(new undici.ProxyAgent(proxy));
  } catch {
    // undici not resolvable — the default fetch dispatcher will be
    // used, which means external fetches that need the proxy will
    // continue to fail. Logging here is suppressed because this runs
    // at module load and would noise up startup; the symptom shows up
    // as fetch failures downstream where it can be diagnosed.
  }
}
