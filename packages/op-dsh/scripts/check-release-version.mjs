import { spawnSync } from 'node:child_process'
import { readFile } from 'node:fs/promises'
import { join } from 'node:path'
import { openPencil, openPencilRoot, platforms, projectRoot } from './platforms.mjs'

const requested = process.argv[2]?.replace(/^v/u, '')
if (requested === undefined) throw new Error('usage: node scripts/check-release-version.mjs <version>')
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/u.test(requested)) {
  throw new Error(`invalid release version ${JSON.stringify(requested)}`)
}

const root = JSON.parse(await readFile(join(projectRoot, 'package.json'), 'utf8'))
if (root.version !== requested) throw new Error(`tag version ${requested} does not match package.json ${root.version}`)
const expectedOptionals = Object.fromEntries(platforms.map(platform => [platform.packageName, requested]))
if (JSON.stringify(root.optionalDependencies) !== JSON.stringify(expectedOptionals)) {
  throw new Error('root optionalDependencies are not the exact synchronized six-platform set')
}
for (const platform of platforms) {
  const manifest = JSON.parse(await readFile(join(projectRoot, 'npm', platform.id, 'package.json'), 'utf8'))
  if (manifest.version !== requested) throw new Error(`${platform.packageName} is not synchronized to ${requested}`)
}

const lockText = await readFile(join(projectRoot, 'pnpm-lock.yaml'), 'utf8')
for (const platform of platforms) {
  const name = escapeRegExp(platform.packageName)
  const entry = new RegExp(`['"]?${name}['"]?:\\n\\s+specifier: ([^\\s]+)`, 'u').exec(lockText)
  const expectedLink = `link:npm/${platform.id}`
  if (entry?.[1] !== expectedLink) throw new Error(`pnpm-lock.yaml does not link ${platform.packageName} to ${expectedLink}`)
  if (!lockText.includes(`'${platform.packageName}': link:./npm/${platform.id}`)) {
    throw new Error(`pnpm-lock.yaml is missing the development override for ${platform.packageName}`)
  }
}
const workspaceText = await readFile(join(projectRoot, 'pnpm-workspace.yaml'), 'utf8')
const packagesSection = /^packages:\s*\n((?:[ \t]+.*\n)*)/mu.exec(workspaceText)?.[1] ?? ''
if (/npm\/\*/u.test(packagesSection)) throw new Error('npm/* must not be pnpm workspace packages')

const cargo = await readFile(join(openPencilRoot, 'Cargo.toml'), 'utf8')
const cargoVersion = /^\[workspace\.package\][\s\S]*?^version = "([^"]+)"/mu.exec(cargo)?.[1]
if (cargoVersion !== openPencil.version) {
  throw new Error(`platforms.json pins OpenPencil ${openPencil.version}, Cargo.toml reports ${cargoVersion ?? 'nothing'}`)
}
if (process.env.DSH_REQUIRE_HEAD_REVISION === '1') {
  const revision = git(['-C', openPencilRoot, 'rev-parse', 'HEAD']).trim()
  if (revision !== openPencil.revision) {
    throw new Error(`platforms.json pins OpenPencil ${openPencil.revision}, HEAD is ${revision}`)
  }
} else {
  git(['-C', openPencilRoot, 'merge-base', '--is-ancestor', openPencil.revision, 'HEAD'])
}
process.stdout.write(requested + '\n')

function git(args) {
  const result = spawnSync('git', args, { cwd: projectRoot, encoding: 'utf8' })
  if (result.status !== 0) throw new Error(`git ${args.join(' ')} failed:\n${result.stderr || result.stdout}`)
  return result.stdout
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&')
}
