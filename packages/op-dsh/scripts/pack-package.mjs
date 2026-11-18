import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { isAbsolute, join, resolve } from 'node:path'
import { argumentValue, projectRoot } from './platforms.mjs'

const packagePath = argumentValue('--package') ?? '.'
const requestedDestination = argumentValue('--destination') ?? 'artifacts'
const destination = isAbsolute(requestedDestination)
  ? requestedDestination
  : resolve(projectRoot, requestedDestination)
await mkdir(destination, { recursive: true })

const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm'
const result = spawnSync(npm, [
  'pack',
  '--json',
  '--ignore-scripts',
  '--pack-destination',
  destination,
  packagePath,
], {
  cwd: projectRoot,
  encoding: 'utf8',
  env: { ...process.env, npm_config_cache: join(tmpdir(), 'dsh-openpencil-npm-cache') },
  shell: process.platform === 'win32',
})
if (result.status !== 0) throw new Error(result.stderr || result.stdout || `npm pack exited ${result.status}`)
const reports = JSON.parse(result.stdout)
if (!Array.isArray(reports) || reports.length !== 1) throw new Error('npm pack returned an unexpected report')
const report = reports[0]
const tarballPath = join(destination, report.filename)
const bytes = await readFile(tarballPath)
const integrity = `sha512-${createHash('sha512').update(bytes).digest('base64')}`
const shasum = createHash('sha1').update(bytes).digest('hex')
if (report.integrity !== integrity || report.shasum !== shasum) {
  throw new Error(`${report.name}@${report.version}: npm report integrity does not match ${tarballPath}`)
}
await writeFile(join(destination, report.filename + '.json'), JSON.stringify(report, null, 2) + '\n')
process.stdout.write(`${report.name}@${report.version} -> ${tarballPath}\n`)
