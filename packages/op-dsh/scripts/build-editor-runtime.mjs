import { spawnSync } from 'node:child_process'
import { join } from 'node:path'

import {
  collabBootstrapEnvNames,
  validatedCollabBootstrapUrls,
} from './collab-bootstrap-config.mjs'
import { argumentValue, openPencilRoot } from './platforms.mjs'

const target = argumentValue('--target')
const bootstrap = validatedCollabBootstrapUrls()
const args = [
  'build',
  '--manifest-path',
  join(openPencilRoot, 'Cargo.toml'),
  '--locked',
  '--release',
  '-p',
  'op-host-web-server',
]
if (target !== undefined) args.push('--target', target)

const result = spawnSync(process.platform === 'win32' ? 'cargo.exe' : 'cargo', args, {
  cwd: openPencilRoot,
  env: {
    ...process.env,
    [collabBootstrapEnvNames[0]]: bootstrap.cn,
    [collabBootstrapEnvNames[1]]: bootstrap.global,
  },
  stdio: 'inherit',
})
if (result.error !== undefined) throw result.error
if (result.status !== 0) {
  throw new Error(`op-host-web-server release build exited with status ${result.status ?? 'unknown'}`)
}

process.stdout.write(
  `built collaboration-enabled op-host-web-server${target === undefined ? '' : ` for ${target}`}\n`,
)
