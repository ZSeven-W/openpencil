/**
 * Loopback addresses must always bypass proxies.
 *
 * In proxy-heavy environments, localhost requests can be forwarded to the proxy
 * if dev-server or Electron child processes do not have NO_PROXY/no_proxy set
 * correctly. For OpenPencil, that breaks the local SSR and Electron dev chain.
 */

export const loopbackBypassHosts = ['127.0.0.1', 'localhost', '::1'] as const

function splitNoProxy(value?: string): string[] {
  if (!value) return []
  return value
    .split(',')
    .map((entry) => entry.trim())
    .filter(Boolean)
}

/**
 * Merge existing NO_PROXY/no_proxy values and always keep loopback hosts in the bypass list.
 */
export function mergeLoopbackNoProxyValue(value?: string): string {
  const entries = new Set(splitNoProxy(value))
  for (const host of loopbackBypassHosts) {
    entries.add(host)
  }
  return Array.from(entries).join(',')
}

/**
 * Return a new environment object with loopback bypass rules applied.
 *
 * Update both NO_PROXY and no_proxy so tools with different case expectations
 * read the same bypass configuration.
 */
export function withLoopbackNoProxy(
  env: NodeJS.ProcessEnv,
): NodeJS.ProcessEnv {
  const merged = mergeLoopbackNoProxyValue(env.NO_PROXY ?? env.no_proxy)
  return {
    ...env,
    NO_PROXY: merged,
    no_proxy: merged,
  }
}

/**
 * Apply loopback bypass rules to the current process environment in place.
 */
export function ensureLoopbackNoProxy(env: NodeJS.ProcessEnv): void {
  const merged = mergeLoopbackNoProxyValue(env.NO_PROXY ?? env.no_proxy)
  env.NO_PROXY = merged
  env.no_proxy = merged
}
