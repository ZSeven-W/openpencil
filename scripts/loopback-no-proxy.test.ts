import { describe, expect, test } from 'bun:test'

import {
  ensureLoopbackNoProxy,
  mergeLoopbackNoProxyValue,
  withLoopbackNoProxy,
} from './loopback-no-proxy'

describe('loopback no_proxy helpers', () => {
  test('mergeLoopbackNoProxyValue 保留已有条目并补齐本地回环地址', () => {
    expect(mergeLoopbackNoProxyValue('example.com,localhost')).toBe(
      'example.com,localhost,127.0.0.1,::1',
    )
  })

  test('withLoopbackNoProxy 同时写回 NO_PROXY 和 no_proxy', () => {
    const next = withLoopbackNoProxy({
      http_proxy: 'http://127.0.0.1:7897',
      NO_PROXY: 'example.com',
    })

    expect(next.NO_PROXY).toBe('example.com,127.0.0.1,localhost,::1')
    expect(next.no_proxy).toBe(next.NO_PROXY)
  })

  test('ensureLoopbackNoProxy 原地更新现有环境对象', () => {
    const env: NodeJS.ProcessEnv = { no_proxy: 'localhost,example.com' }

    ensureLoopbackNoProxy(env)

    expect(env.NO_PROXY).toBe('localhost,example.com,127.0.0.1,::1')
    expect(env.no_proxy).toBe(env.NO_PROXY)
  })
})
