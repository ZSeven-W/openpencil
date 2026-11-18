import { readFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

export const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
export const openPencilRoot = resolve(
  process.env.OPENPENCIL_SOURCE_ROOT?.trim()
  || process.env.OPENPENCIL_ROOT?.trim()
  || resolve(projectRoot, '..', '..'),
)

const config = JSON.parse(await readFile(join(projectRoot, 'platforms.json'), 'utf8'))
if (config.schemaVersion !== 1) throw new Error('platforms.json: unsupported schemaVersion')
if (!config.openPencil || typeof config.openPencil !== 'object') {
  throw new Error('platforms.json: openPencil metadata is required')
}
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/u.test(config.openPencil.version ?? '')) {
  throw new Error('platforms.json: invalid OpenPencil version')
}
if (!/^[0-9a-f]{40}$/u.test(config.openPencil.revision ?? '')) {
  throw new Error('platforms.json: OpenPencil revision must be a full lowercase commit SHA')
}
if (!Array.isArray(config.platforms) || config.platforms.length !== 6) {
  throw new Error('platforms.json: exactly six platforms are required')
}

export const openPencil = Object.freeze({ ...config.openPencil })
export const platforms = Object.freeze(config.platforms.map(platform => Object.freeze({ ...platform })))

const expectedIds = [
  'darwin-arm64',
  'darwin-x64',
  'linux-arm64',
  'linux-x64',
  'win32-arm64',
  'win32-x64',
]
if (platforms.some((platform, index) => platform.id !== expectedIds[index])) {
  throw new Error(`platforms.json: expected platform order ${expectedIds.join(', ')}`)
}
if (new Set(platforms.map(platform => platform.packageName)).size !== platforms.length) {
  throw new Error('platforms.json: platform package names must be unique')
}

export const runtimeManifestName = 'openpencil-runtime.json'

export function platformById(id) {
  const platform = platforms.find(candidate => candidate.id === id)
  if (platform === undefined) {
    throw new Error(`unsupported platform ${JSON.stringify(id)}; expected one of ${expectedIds.join(', ')}`)
  }
  return platform
}

export function platformPackageRoot(platform) {
  return join(projectRoot, 'npm', platform.id)
}

export function runtimeKeyPaths(platform) {
  return [
    `bin/${platform.binaryName}`,
    'bin/web-bundle/op_host_web.js',
    'bin/web-bundle/op_host_web_bg.wasm',
    'bin/web-bundle/assets/iconify-catalog-core.json',
    'bin/web-bundle/canvaskit/canvaskit.js',
    'bin/web-bundle/canvaskit/canvaskit.wasm',
  ]
}

export function argumentValue(name) {
  const exactIndex = process.argv.indexOf(name)
  if (exactIndex !== -1) {
    const value = process.argv[exactIndex + 1]
    if (value === undefined || value.startsWith('--')) throw new Error(`${name} requires a value`)
    return value
  }
  const prefix = name + '='
  const joined = process.argv.find(argument => argument.startsWith(prefix))
  return joined?.slice(prefix.length)
}

export function hasFlag(name) {
  return process.argv.includes(name)
}
