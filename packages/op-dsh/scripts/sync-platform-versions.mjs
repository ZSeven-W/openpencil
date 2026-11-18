import { spawnSync } from 'node:child_process'
import { readFile, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { platforms, projectRoot } from './platforms.mjs'

const requested = process.argv[2]?.replace(/^v/u, '')
const rootPath = join(projectRoot, 'package.json')
const rootManifest = JSON.parse(await readFile(rootPath, 'utf8'))
const version = requested ?? rootManifest.version
if (typeof version !== 'string' || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/u.test(version)) {
  throw new Error(`invalid release version ${JSON.stringify(version)}`)
}

rootManifest.version = version
rootManifest.optionalDependencies = Object.fromEntries(
  platforms.map(platform => [platform.packageName, version]),
)
await writeFile(rootPath, JSON.stringify(rootManifest, null, 2) + '\n')

for (const platform of platforms) {
  const manifestPath = join(projectRoot, 'npm', platform.id, 'package.json')
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8'))
  manifest.version = version
  await writeFile(manifestPath, JSON.stringify(manifest, null, 2) + '\n')
}

const pnpm = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm'
const lock = spawnSync(pnpm, ['install', '--lockfile-only', '--ignore-scripts'], {
  cwd: projectRoot,
  encoding: 'utf8',
  shell: process.platform === 'win32',
})
if (lock.status !== 0) {
  throw new Error(`platform manifests were updated, but pnpm-lock.yaml refresh failed:\n${lock.stderr || lock.stdout}`)
}
process.stdout.write(`synchronized root, six runtime packages, and pnpm-lock.yaml to ${version}\n`)
