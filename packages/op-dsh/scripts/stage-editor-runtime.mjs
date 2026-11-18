import { createHash } from 'node:crypto'
import {
  chmod,
  copyFile,
  lstat,
  mkdir,
  readdir,
  readFile,
  rm,
  writeFile,
} from 'node:fs/promises'
import { basename, dirname, isAbsolute, join, relative, resolve, sep } from 'node:path'
import { pathToFileURL } from 'node:url'
import {
  argumentValue,
  openPencil,
  openPencilRoot,
  platformById,
  platformPackageRoot,
  projectRoot,
  runtimeKeyPaths,
  runtimeManifestName,
} from './platforms.mjs'

if (isMain(import.meta.url)) await main()

async function main() {
  const requestedPlatform = argumentValue('--platform')
  const id = requestedPlatform ?? `${process.platform}-${process.arch}`
  const platform = platformById(id)
  const vendorRoot = openPencilRoot
  const binarySource = await sourcePath(
    '--binary',
    editorBinarySourceCandidates(vendorRoot, platform, requestedPlatform === undefined),
    'binary',
  )
  const pkgSource = await resolveWebBundleSource(vendorRoot, argumentValue('--pkg'))
  const canvasKitSource = await sourcePath('--canvaskit', [
    join(vendorRoot, 'crates', 'op-host-web', 'assets', 'canvaskit'),
  ], 'CanvasKit directory')
  const packageRoot = platformPackageRoot(platform)

  await assertRegularFile(binarySource, '--binary')
  await assertDirectory(pkgSource, '--pkg')
  await assertDirectory(canvasKitSource, '--canvaskit')
  for (const source of [binarySource, pkgSource, canvasKitSource]) {
    if (within(source, packageRoot)) {
      throw new Error(`staging source must not be inside generated package runtime: ${source}`)
    }
  }

  const binaryDestination = join(packageRoot, 'bin', platform.binaryName)
  const pkgDestination = join(packageRoot, 'bin', 'web-bundle')
  const canvasKitDestination = join(pkgDestination, 'canvaskit')
  await rm(join(packageRoot, 'bin'), { recursive: true, force: true })
  // Delete the legacy layout so generated packages never retain a second
  // copy of the web runtime beside the daemon-native deploy layout.
  await rm(join(packageRoot, 'web'), { recursive: true, force: true })
  await rm(join(packageRoot, runtimeManifestName), { force: true })
  await mkdir(dirname(binaryDestination), { recursive: true })
  await copyFile(binarySource, binaryDestination)
  if (platform.os !== 'win32') await chmod(binaryDestination, 0o755)
  await copyDirectory(pkgSource, pkgDestination, shouldOmitWebFile)
  await copyDirectory(canvasKitSource, canvasKitDestination, shouldOmitWebFile)

  for (const sourceName of ['LICENSE', 'THIRD_PARTY_NOTICES.md']) {
    await copyFile(join(projectRoot, sourceName), join(packageRoot, sourceName))
  }

  for (const path of runtimeKeyPaths(platform)) {
    await assertRegularFile(join(packageRoot, ...path.split('/')), path)
  }
  const files = {}
  for (const path of await collectRuntimePayloadPaths(packageRoot)) {
    files[path] = await sha256(join(packageRoot, ...path.split('/')))
  }
  const manifest = {
    schemaVersion: 1,
    platform: platform.id,
    packageName: platform.packageName,
    openPencilVersion: openPencil.version,
    openPencilRevision: openPencil.revision,
    files,
  }
  await writeFile(join(packageRoot, runtimeManifestName), JSON.stringify(manifest, null, 2) + '\n')
  process.stdout.write(`staged ${platform.packageName} from OpenPencil ${openPencil.version} (${openPencil.revision})\n`)
}

/**
 * Match the default build command: a host build without `--target` lands in
 * target/release, while an explicitly selected platform normally lands in
 * target/<rustTarget>/release. This prevents an old cross-target artifact
 * from silently shadowing a freshly built local daemon.
 */
export function editorBinarySourceCandidates(vendorRoot, platform, preferHostBuild) {
  const host = join(vendorRoot, 'target', 'release', platform.binaryName)
  const targeted = join(vendorRoot, 'target', platform.rustTarget, 'release', platform.binaryName)
  return preferHostBuild ? [host, targeted] : [targeted, host]
}

export async function resolveWebBundleSource(vendorRoot, explicitPkg) {
  return sourcePath('--pkg', [
    join(vendorRoot, 'crates', 'op-host-web', 'pkg'),
  ], 'current web bundle', explicitPkg)
}

export async function collectRuntimePayloadPaths(packageRoot) {
  const roots = [join(packageRoot, 'bin')]
  const files = (await Promise.all(roots.map(walkRegularFiles))).flat()
  return files
    .map(path => relative(packageRoot, path).split(sep).join('/'))
    .sort((left, right) => left.localeCompare(right, 'en'))
}

async function sourcePath(name, defaults, label, explicitValue) {
  const value = explicitValue ?? argumentValue(name)
  if (value !== undefined) return resolve(value)
  for (const candidate of defaults) {
    try {
      await lstat(candidate)
      return candidate
    } catch (error) {
      if (error?.code !== 'ENOENT') throw error
    }
  }
  throw new Error(`no default ${label} found; checked ${defaults.join(', ')} (override with ${name})`)
}

function isMain(url) {
  const entry = process.argv[1]
  return entry !== undefined && pathToFileURL(resolve(entry)).href === url
}

function within(candidate, parent) {
  const path = relative(resolve(parent), resolve(candidate))
  return path === '' || (!path.startsWith('..' + sep) && path !== '..' && !isAbsolute(path))
}

function shouldOmitWebFile(path) {
  const name = basename(path)
  return name.endsWith('.d.ts') || name.endsWith('.opt.wasm')
}

async function copyDirectory(source, destination, omit) {
  await mkdir(destination, { recursive: true })
  const entries = await readdir(source, { withFileTypes: true })
  entries.sort((left, right) => left.name.localeCompare(right.name, 'en'))
  for (const entry of entries) {
    const sourcePath = join(source, entry.name)
    if (omit(sourcePath)) continue
    const destinationPath = join(destination, entry.name)
    if (entry.isSymbolicLink()) throw new Error(`runtime source must not contain symbolic links: ${sourcePath}`)
    if (entry.isDirectory()) await copyDirectory(sourcePath, destinationPath, omit)
    else if (entry.isFile()) await copyFile(sourcePath, destinationPath)
    else throw new Error(`unsupported runtime source entry: ${sourcePath}`)
  }
}

async function walkRegularFiles(root) {
  const output = []
  const entries = await readdir(root, { withFileTypes: true })
  entries.sort((left, right) => left.name.localeCompare(right.name, 'en'))
  for (const entry of entries) {
    const path = join(root, entry.name)
    if (entry.isSymbolicLink()) throw new Error(`runtime payload must not contain symbolic links: ${path}`)
    if (entry.isDirectory()) output.push(...await walkRegularFiles(path))
    else if (entry.isFile()) output.push(path)
    else throw new Error(`unsupported runtime payload entry: ${path}`)
  }
  return output
}

async function assertDirectory(path, label) {
  let value
  try {
    value = await lstat(path)
  } catch (error) {
    if (error?.code === 'ENOENT') throw new Error(`${label} directory is missing: ${path}`)
    throw error
  }
  if (!value.isDirectory() || value.isSymbolicLink()) throw new Error(`${label} must be a real directory: ${path}`)
}

async function assertRegularFile(path, label) {
  let value
  try {
    value = await lstat(path)
  } catch (error) {
    if (error?.code === 'ENOENT') throw new Error(`${label} file is missing: ${path}`)
    throw error
  }
  if (!value.isFile() || value.isSymbolicLink()) throw new Error(`${label} must be a regular file: ${path}`)
}

async function sha256(path) {
  return createHash('sha256').update(await readFile(path)).digest('hex')
}
