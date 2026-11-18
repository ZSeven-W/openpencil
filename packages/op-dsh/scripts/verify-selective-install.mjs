import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { mkdtemp, readFile, readdir, realpath, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { basename, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import { openPencil, platforms } from './platforms.mjs'

const artifactRoot = resolve(process.argv[2] ?? 'artifacts')
const reports = await readReports(artifactRoot)
const expectedNames = new Set(['@zseven-w/dsh-openpencil', ...platforms.map(platform => platform.packageName)])
if (reports.length !== expectedNames.size) {
  throw new Error(`expected exactly ${expectedNames.size} package reports, found ${reports.length}`)
}
const reportsByName = new Map()
for (const report of reports) {
  if (!expectedNames.has(report.name)) throw new Error(`unexpected package report ${report.name}`)
  if (reportsByName.has(report.name)) throw new Error(`duplicate package report ${report.name}`)
  await verifyReportIntegrity(report)
  reportsByName.set(report.name, report)
}
const rootReport = reportsByName.get('@zseven-w/dsh-openpencil')
if (rootReport === undefined) throw new Error('root package tarball is missing')
for (const report of reports) {
  if (report.version !== rootReport.version) throw new Error(`${report.name} version differs from root ${rootReport.version}`)
}

const optionalDependencies = Object.fromEntries(platforms.map(platform => {
  const report = reportsByName.get(platform.packageName)
  if (report === undefined) throw new Error(`${platform.packageName} tarball is missing`)
  return [platform.packageName, pathToFileURL(join(report.directory, report.filename)).href]
}))

for (const platform of platforms) await verifySelection(platform, optionalDependencies, rootReport)
for (const platform of platforms.filter(platform => platform.os === 'linux')) {
  await verifyUnsupportedLibc(platform, optionalDependencies, rootReport)
}
await verifyOmitted(optionalDependencies, rootReport)
process.stdout.write('verified npm selective installation for six atomic runtimes, Linux musl rejection, and --omit=optional\n')

async function verifySelection(platform, dependencies, root) {
  const directory = await createConsumer(dependencies, root)
  try {
    const args = [
      'install',
      '--ignore-scripts',
      '--package-lock=false',
      '--no-audit',
      '--no-fund',
      '--legacy-peer-deps',
      '--offline',
      `--os=${platform.os}`,
      `--cpu=${platform.cpu}`,
    ]
    if (platform.libc !== undefined) args.push(`--libc=${platform.libc}`)
    runNpm(args, directory)
    const installed = await installedRuntimePackages(directory)
    const expected = basename(platform.packageName)
    if (installed.length !== 1 || installed[0] !== expected) {
      throw new Error(`${platform.id}: expected ${expected}, installed ${installed.join(', ') || 'nothing'}`)
    }
    const runtime = resolveInstalledRuntime(directory, platform)
    const packageRoot = await realpath(join(directory, 'node_modules', '@zseven-w', expected))
    const expectedRuntime = {
      binary: join(packageRoot, 'bin', platform.binaryName),
      webBundleDir: join(packageRoot, 'bin', 'web-bundle'),
      canvasKitDir: join(packageRoot, 'bin', 'web-bundle', 'canvaskit'),
      openPencilVersion: openPencil.version,
      revision: openPencil.revision,
      source: 'optional-package',
    }
    if (JSON.stringify(runtime) !== JSON.stringify(expectedRuntime)) {
      throw new Error(`${platform.id}: root resolver selected ${JSON.stringify(runtime)}, expected ${JSON.stringify(expectedRuntime)}`)
    }
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
}

async function verifyUnsupportedLibc(platform, dependencies, root) {
  const directory = await createConsumer(dependencies, root)
  try {
    runNpm([
      'install',
      '--ignore-scripts',
      '--package-lock=false',
      '--no-audit',
      '--no-fund',
      '--legacy-peer-deps',
      '--offline',
      `--os=${platform.os}`,
      `--cpu=${platform.cpu}`,
      '--libc=musl',
    ], directory)
    const installed = await installedRuntimePackages(directory)
    if (installed.length !== 0) {
      throw new Error(`${platform.id}/musl: expected no glibc runtime, installed ${installed.join(', ')}`)
    }
    if (tryResolveInstalledRuntime(directory, platform) !== undefined) {
      throw new Error(`${platform.id}/musl: resolver unexpectedly found a runtime`)
    }
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
}

async function verifyOmitted(dependencies, root) {
  const directory = await createConsumer(dependencies, root)
  try {
    runNpm([
      'install',
      '--ignore-scripts',
      '--package-lock=false',
      '--no-audit',
      '--no-fund',
      '--legacy-peer-deps',
      '--offline',
      '--omit=optional',
    ], directory)
    const installed = await installedRuntimePackages(directory)
    if (installed.length !== 0) throw new Error(`--omit=optional installed ${installed.join(', ')}`)
    if (tryResolveInstalledRuntime(directory, platforms[0]) !== undefined) {
      throw new Error('--omit=optional unexpectedly resolved an editor runtime')
    }
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
}

async function createConsumer(optionalDependencies, root) {
  const directory = await mkdtemp(join(tmpdir(), 'dsh-openpencil-install-'))
  await writeFile(join(directory, 'package.json'), JSON.stringify({
    name: 'dsh-openpencil-install-smoke',
    private: true,
    version: '0.0.0',
    dependencies: {
      '@zseven-w/dsh-openpencil': pathToFileURL(join(root.directory, root.filename)).href,
    },
    optionalDependencies,
  }, null, 2) + '\n')
  return directory
}

function runNpm(args, cwd) {
  const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm'
  const result = spawnSync(npm, args, {
    cwd,
    encoding: 'utf8',
    env: { ...process.env, npm_config_cache: join(tmpdir(), 'dsh-openpencil-npm-cache') },
    shell: process.platform === 'win32',
  })
  if (result.status !== 0) throw new Error(`npm ${args.join(' ')} failed:\n${result.stderr || result.stdout}`)
}

function resolveInstalledRuntime(directory, platform) {
  const output = runResolver(directory, platform, false)
  if (output === '') throw new Error(`${platform.id}: root resolver did not find the installed runtime`)
  return JSON.parse(output)
}

function tryResolveInstalledRuntime(directory, platform) {
  const output = runResolver(directory, platform, true)
  return output === '' ? undefined : JSON.parse(output)
}

function runResolver(directory, platform, optional) {
  const method = optional ? 'tryResolveEditorRuntime' : 'resolveEditorRuntime'
  const source = [
    "const { dirname, join } = require('node:path')",
    "const { pathToFileURL } = require('node:url')",
    "const manifest = require.resolve('@zseven-w/dsh-openpencil/package.json', { paths: [process.cwd()] })",
    "import(pathToFileURL(join(dirname(manifest), 'lib', 'editor-runtime.js')).href).then(mod => {",
    `  const value = mod.${method}({ platform: ${JSON.stringify(platform.os)}, arch: ${JSON.stringify(platform.cpu)}, env: {} })`,
    "  process.stdout.write(value === undefined ? '' : JSON.stringify(value))",
    "}).catch(error => { console.error(error); process.exitCode = 1 })",
  ].join(';')
  const result = spawnSync(process.execPath, ['-e', source], { cwd: directory, encoding: 'utf8' })
  if (result.status !== 0) throw new Error(`root resolver failed for ${platform.id}:\n${result.stderr || result.stdout}`)
  return result.stdout.trim()
}

async function installedRuntimePackages(directory) {
  try {
    return (await readdir(join(directory, 'node_modules', '@zseven-w')))
      .filter(name => name.startsWith('dsh-openpencil-') && name !== 'dsh-openpencil')
      .sort()
  } catch {
    return []
  }
}

async function verifyReportIntegrity(report) {
  const tarball = await readFile(join(report.directory, report.filename))
  const integrity = `sha512-${createHash('sha512').update(tarball).digest('base64')}`
  const shasum = createHash('sha1').update(tarball).digest('hex')
  if (report.integrity !== integrity || report.shasum !== shasum) {
    throw new Error(`${report.name}@${report.version}: report integrity does not match tarball`)
  }
}

async function readReports(directory) {
  const output = []
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name)
    if (entry.isDirectory()) output.push(...await readReports(path))
    else if (entry.name.endsWith('.tgz.json')) {
      output.push({ ...JSON.parse(await readFile(path, 'utf8')), directory })
    }
  }
  return output
}
