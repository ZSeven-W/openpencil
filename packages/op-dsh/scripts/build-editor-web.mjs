import { spawnSync } from 'node:child_process'

import { withoutCollabBootstrapBuildEnv } from './collab-bootstrap-config.mjs'
import { openPencilRoot } from './platforms.mjs'

const result = spawnSync('bash', ['tools/check-wasm-bundle.sh'], {
  cwd: openPencilRoot,
  env: withoutCollabBootstrapBuildEnv(process.env),
  stdio: 'inherit',
})

if (result.error !== undefined) {
  if (result.error.code === 'ENOENT') {
    throw new Error('cannot build the OpenPencil editor web bundle: bash was not found on PATH')
  }
  throw new Error(`failed to start the OpenPencil editor web bundle build: ${result.error.message}`)
}
if (result.signal !== null) {
  throw new Error(`OpenPencil editor web bundle build was terminated by signal ${result.signal}`)
}
if (result.status !== 0) {
  throw new Error(
    `OpenPencil editor web bundle build exited with status ${result.status ?? 'unknown'}; see the gate output above`,
  )
}

process.stdout.write('built and verified the OpenPencil editor web bundle\n')
