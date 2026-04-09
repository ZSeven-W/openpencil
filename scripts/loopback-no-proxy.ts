/**
 * 本地回环地址必须直连。
 *
 * 在带代理环境里, 如果 dev server 或 Electron 子进程没有正确补齐
 * NO_PROXY/no_proxy, 那么发往 localhost 的请求可能会被错误转发给代理。
 * 对 OpenPencil 来说, 这会直接打断本地 SSR 和 Electron dev 的加载链路。
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
 * 合并已有的 NO_PROXY/no_proxy, 并确保本地回环地址永远在绕过名单里。
 */
export function mergeLoopbackNoProxyValue(value?: string): string {
  const entries = new Set(splitNoProxy(value))
  for (const host of loopbackBypassHosts) {
    entries.add(host)
  }
  return Array.from(entries).join(',')
}

/**
 * 返回一个带本地回环绕过规则的新环境变量对象。
 *
 * 这里同时写回 NO_PROXY 和 no_proxy。
 * 这样可以兼容不同工具对大小写环境变量的读取习惯。
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
 * 原地补齐当前进程的本地回环绕过规则。
 */
export function ensureLoopbackNoProxy(env: NodeJS.ProcessEnv): void {
  const merged = mergeLoopbackNoProxyValue(env.NO_PROXY ?? env.no_proxy)
  env.NO_PROXY = merged
  env.no_proxy = merged
}
