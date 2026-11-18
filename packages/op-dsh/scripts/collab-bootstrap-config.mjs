import { spawnSync } from 'node:child_process'
import { join } from 'node:path'

import { openPencilRoot, projectRoot } from './platforms.mjs'

export const collabBootstrapEnvNames = Object.freeze([
  'OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN',
  'OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL',
])

export function withoutCollabBootstrapBuildEnv(env = process.env) {
  const sanitized = { ...env }
  for (const name of collabBootstrapEnvNames) delete sanitized[name]
  return sanitized
}

const validator = join(
  openPencilRoot,
  'tools',
  'check-collab-bootstrap-urls.py',
)

/**
 * Read and validate both production relay bootstrap URLs without putting their
 * values on a command line. OpenPencil deliberately keeps these endpoints out
 * of the open-source binary; every published native package must inject both.
 */
export function validatedCollabBootstrapUrls(env = process.env) {
  const values = collabBootstrapEnvNames.map(name => {
    const value = env[name]
    if (typeof value !== 'string' || value.length === 0) {
      throw new Error(`${name} is required for a collaboration-enabled editor runtime`)
    }
    return value
  })

  const payload = Buffer.from(`${values[0]}\0${values[1]}\0`, 'utf8')
  const candidates = process.platform === 'win32'
    ? [['python', []], ['py', ['-3']], ['python3', []]]
    : [['python3', []], ['python', []]]

  for (const [command, prefix] of candidates) {
    const result = spawnSync(command, [...prefix, validator], {
      cwd: projectRoot,
      env: withoutCollabBootstrapBuildEnv(process.env),
      input: payload,
      encoding: 'utf8',
      maxBuffer: 64 * 1024,
    })
    if (result.error?.code === 'ENOENT') continue
    if (result.error !== undefined) {
      throw new Error(`could not run the OpenPencil relay bootstrap validator with ${command}`, {
        cause: result.error,
      })
    }
    if (result.status !== 0) {
      const detail = result.stderr.trim() || 'OpenPencil relay bootstrap validation failed'
      throw new Error(detail)
    }
    return Object.freeze({
      cn: values[0],
      global: values[1],
    })
  }

  throw new Error('Python 3 is required to validate the production collaboration bootstrap URLs')
}
