import { setGlobalDispatcher, EnvHttpProxyAgent } from 'undici';

/**
 * Configure undici's global fetch dispatcher to honor HTTPS_PROXY /
 * HTTP_PROXY / NO_PROXY env vars. Node's native `fetch` (built on
 * undici) does NOT auto-route through the system proxy the way `curl`
 * does — it goes direct, which fails with `ECONNREFUSED` on machines
 * that route outbound HTTPS through a local proxy (clash / mihomo /
 * corporate gateway). The image-search endpoint already swallows that
 * failure and silently falls back to Wikimedia, but Wikimedia is
 * blocked on the same machines, so designs land with empty image
 * placeholders.
 *
 * `EnvHttpProxyAgent` is undici's built-in env-aware dispatcher: it
 * reads HTTPS_PROXY / HTTP_PROXY / NO_PROXY (case-insensitive)
 * directly from `process.env`, applies the bypass list to no-proxy
 * hosts, and routes the rest through the configured proxy. When no
 * proxy env var is set, requests pass through unchanged (production
 * deploys, CI), so this is a safe no-op there.
 *
 * Why static ESM import (not `require('undici')`):
 *   The previous version did `require('undici')` inside a try/catch,
 *   thinking that would let it run on both CJS and ESM. In an ESM
 *   module (which is what Vite/Nitro produces in dev) `require` is
 *   undefined and the call threw a ReferenceError that got caught and
 *   silenced — meaning the proxy was never installed, and the
 *   image-search endpoint kept ECONNREFUSED-ing on proxied dev
 *   machines. undici ships inside Node 18+ itself (Node's fetch is
 *   built on it) and is always resolvable as an ESM module, so a
 *   static import is the right shape.
 *
 * Idempotent: first call installs the dispatcher, subsequent calls
 * are no-ops, so endpoints can call this from their own module init
 * without coordinating.
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

  setGlobalDispatcher(new EnvHttpProxyAgent());
}
