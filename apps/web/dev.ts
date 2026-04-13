import { spawn } from 'node:child_process'
import { join } from 'node:path'

import { withLoopbackNoProxy } from '../../scripts/loopback-no-proxy'

const VITE_CLI = join(import.meta.dirname, '..', '..', 'node_modules', 'vite', 'bin', 'vite.js')

/**
 * 在真正启动 Vite 前先补齐本地回环地址的代理绕过规则。
 *
 * 这一步必须发生在 Vite 进程启动之前。
 * 否则带代理环境下, SSR 文档请求仍可能被错误送去代理并返回 502。
 */
const child = spawn('node', [VITE_CLI, 'dev', '--port', '3000'], {
  cwd: import.meta.dirname,
  stdio: 'inherit',
  env: withLoopbackNoProxy(process.env),
})

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal)
    return
  }
  process.exit(code ?? 0)
})
