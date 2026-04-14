import { spawn } from 'node:child_process'
import { join } from 'node:path'

import { withLoopbackNoProxy } from '../../scripts/loopback-no-proxy'

const VITE_CLI = join(import.meta.dirname, '..', '..', 'node_modules', 'vite', 'bin', 'vite.js')

/**
 * Ensure loopback proxy bypass rules are in place before Vite starts.
 *
 * This must happen before the Vite process launches.
 * Otherwise SSR document requests can still be routed through a proxy and return 502.
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
