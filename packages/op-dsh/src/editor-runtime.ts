/**
 * Atomic OpenPencil editor runtime resolution.
 *
 * A runtime is always selected as one complete package root. The daemon and
 * both web asset directories are never assembled from unrelated installs.
 */
import { createHash } from 'node:crypto'
import { chmodSync, readFileSync, statSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, isAbsolute, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { pluginEnv, pluginEnvName } from './plugin-env.js'

export const EDITOR_RUNTIME_SCHEMA_VERSION = 1

/** Suffixes of the three editor-runtime overrides — see plugin-env.ts. */
export const EDITOR_RUNTIME_ENV_SUFFIX = Object.freeze({
  binary: 'OPENPENCIL_EDITOR_BINARY',
  webBundleDir: 'OPENPENCIL_EDITOR_WEB_BUNDLE_DIR',
  canvasKitDir: 'OPENPENCIL_EDITOR_CANVASKIT_DIR',
})

/** Current names of those overrides; the legacy DSH_ names are still read. */
export const EDITOR_RUNTIME_ENV = Object.freeze({
  binary: pluginEnvName(EDITOR_RUNTIME_ENV_SUFFIX.binary),
  webBundleDir: pluginEnvName(EDITOR_RUNTIME_ENV_SUFFIX.webBundleDir),
  canvasKitDir: pluginEnvName(EDITOR_RUNTIME_ENV_SUFFIX.canvasKitDir),
})

export type EditorRuntimeSource = 'override' | 'optional-package' | 'development-package'

export interface EditorRuntime {
  binary: string
  webBundleDir: string
  canvasKitDir: string
  openPencilVersion: string
  revision: string
  source: EditorRuntimeSource
}

export type EditorRuntimeLayout = 'canonical' | 'legacy'

export interface EditorRuntimePlatform {
  id: string
  os: string
  cpu: string
  libc?: string
  rustTarget: string
  runner: string
  packageName: string
  binaryName: string
}

export interface EditorRuntimeCatalog {
  schemaVersion: 1
  openPencil: {
    version: string
    revision: string
  }
  platforms: readonly EditorRuntimePlatform[]
}

export interface EditorRuntimeManifest {
  schemaVersion: 1
  platform: string
  packageName: string
  openPencilVersion: string
  openPencilRevision: string
  files: Readonly<Record<string, string>>
}

export type EditorRuntimeErrorCode =
  | 'catalog-unavailable'
  | 'invalid-catalog'
  | 'unsupported-platform'
  | 'partial-override'
  | 'invalid-override'
  | 'runtime-not-found'
  | 'invalid-runtime'

/** A stable, machine-readable error with concise operator diagnostics. */
export class EditorRuntimeUnavailableError extends Error {
  constructor(
    readonly code: EditorRuntimeErrorCode,
    message: string,
    readonly diagnostics: readonly string[] = [],
    options?: ErrorOptions,
  ) {
    super(message, options)
    this.name = 'EditorRuntimeUnavailableError'
  }
}

interface FileStat {
  isFile(): boolean
  isDirectory(): boolean
}

export interface ResolveEditorRuntimeOptions {
  platform?: string
  arch?: string
  env?: Readonly<Record<string, string | undefined>>
  /** Repository/package root containing platforms.json and npm/<platform>. */
  projectRoot?: string
  /** Inject parsed catalog data to make resolution independent of disk state. */
  catalog?: unknown
  resolvePackageJson?: (specifier: string) => string
  readFile?: (path: string) => Buffer
  stat?: (path: string) => FileStat
  chmod?: (path: string, mode: number) => void
}

interface RuntimeExpectation {
  platform: EditorRuntimePlatform
  version: string
  revision: string
}

interface RuntimeIo {
  readFile(path: string): Buffer
  stat(path: string): FileStat
  chmod(path: string, mode: number): void
}

const requireFromPlugin = createRequire(import.meta.url)
const SHA256_PATTERN = /^[a-f0-9]{64}$/
const REVISION_PATTERN = /^[a-f0-9]{40,64}$/
const VERSION_PATTERN = /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/
const SAFE_ID_PATTERN = /^[a-z0-9][a-z0-9-]*$/

function recordFrom(value: unknown): Record<string, unknown> | undefined {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
    ? value as Record<string, unknown>
    : undefined
}

function nonEmptyString(value: unknown): value is string {
  return typeof value === 'string' && value.trim() === value && value.length > 0
}

function catalogError(message: string): never {
  throw new EditorRuntimeUnavailableError('invalid-catalog', `Invalid OpenPencil runtime catalog: ${message}`)
}

/** Parse and validate platforms.json without performing any I/O. */
export function parseEditorRuntimeCatalog(value: unknown): EditorRuntimeCatalog {
  const catalog = recordFrom(value)
  if (catalog === undefined) catalogError('root must be an object')
  if (catalog.schemaVersion !== EDITOR_RUNTIME_SCHEMA_VERSION) {
    catalogError(`schemaVersion must be ${EDITOR_RUNTIME_SCHEMA_VERSION}`)
  }

  const openPencil = recordFrom(catalog.openPencil)
  if (openPencil === undefined) catalogError('openPencil must be an object')
  if (!nonEmptyString(openPencil.version) || !VERSION_PATTERN.test(openPencil.version)) {
    catalogError('openPencil.version must be a semantic version')
  }
  if (!nonEmptyString(openPencil.revision) || !REVISION_PATTERN.test(openPencil.revision)) {
    catalogError('openPencil.revision must be a full hexadecimal Git revision')
  }
  if (!Array.isArray(catalog.platforms) || catalog.platforms.length === 0) {
    catalogError('platforms must be a non-empty array')
  }

  const platforms: EditorRuntimePlatform[] = []
  const ids = new Set<string>()
  const selectors = new Set<string>()
  const packageNames = new Set<string>()
  for (const [index, rawPlatform] of catalog.platforms.entries()) {
    const entry = recordFrom(rawPlatform)
    if (entry === undefined) catalogError(`platforms[${index}] must be an object`)
    for (const field of ['id', 'os', 'cpu', 'rustTarget', 'runner', 'packageName', 'binaryName'] as const) {
      if (!nonEmptyString(entry[field])) catalogError(`platforms[${index}].${field} must be a non-empty string`)
    }
    if (!SAFE_ID_PATTERN.test(entry.id as string)) catalogError(`platforms[${index}].id is invalid`)
    if (!SAFE_ID_PATTERN.test(entry.os as string)) catalogError(`platforms[${index}].os is invalid`)
    if (!SAFE_ID_PATTERN.test(entry.cpu as string)) catalogError(`platforms[${index}].cpu is invalid`)
    if (entry.libc !== undefined && !nonEmptyString(entry.libc)) {
      catalogError(`platforms[${index}].libc must be a non-empty string when present`)
    }

    const platform = entry as unknown as EditorRuntimePlatform
    const selector = `${platform.os}-${platform.cpu}`
    if (platform.id !== selector) catalogError(`platforms[${index}].id must equal ${selector}`)
    const expectedPackageName = `@zseven-w/dsh-openpencil-${platform.id}`
    if (platform.packageName !== expectedPackageName) {
      catalogError(`platforms[${index}].packageName must equal ${expectedPackageName}`)
    }
    const expectedBinaryName = platform.os === 'win32'
      ? 'op-host-web-server.exe'
      : 'op-host-web-server'
    if (platform.binaryName !== expectedBinaryName) {
      catalogError(`platforms[${index}].binaryName must equal ${expectedBinaryName}`)
    }
    if (ids.has(platform.id)) catalogError(`duplicate platform id ${platform.id}`)
    if (selectors.has(selector)) catalogError(`duplicate platform selector ${selector}`)
    if (packageNames.has(platform.packageName)) catalogError(`duplicate packageName ${platform.packageName}`)
    ids.add(platform.id)
    selectors.add(selector)
    packageNames.add(platform.packageName)
    platforms.push(Object.freeze({ ...platform }))
  }

  return Object.freeze({
    schemaVersion: EDITOR_RUNTIME_SCHEMA_VERSION,
    openPencil: Object.freeze({
      version: openPencil.version as string,
      revision: openPencil.revision as string,
    }),
    platforms: Object.freeze(platforms),
  })
}

function safeManifestPath(path: string): boolean {
  if (!nonEmptyString(path) || isAbsolute(path) || path.includes('\\')) return false
  const segments = path.split('/')
  return segments.every(segment => segment !== '' && segment !== '.' && segment !== '..')
}

/** Parse one openpencil-runtime.json and bind it to its expected release. */
export function parseEditorRuntimeManifest(
  value: unknown,
  expected: Readonly<{
    platform: string
    packageName: string
    openPencilVersion: string
    openPencilRevision: string
  }>,
): EditorRuntimeManifest {
  const fail = (message: string): never => {
    throw new EditorRuntimeUnavailableError('invalid-runtime', `Invalid OpenPencil runtime manifest: ${message}`)
  }
  const rawManifest = recordFrom(value)
  if (rawManifest === undefined) fail('root must be an object')
  const manifest = rawManifest as Record<string, unknown>
  if (manifest.schemaVersion !== EDITOR_RUNTIME_SCHEMA_VERSION) {
    fail(`schemaVersion must be ${EDITOR_RUNTIME_SCHEMA_VERSION}`)
  }
  if (manifest.platform !== expected.platform) fail(`platform must equal ${expected.platform}`)
  if (manifest.packageName !== expected.packageName) fail(`packageName must equal ${expected.packageName}`)
  if (manifest.openPencilVersion !== expected.openPencilVersion) {
    fail(`openPencilVersion must equal ${expected.openPencilVersion}`)
  }
  if (manifest.openPencilRevision !== expected.openPencilRevision) {
    fail(`openPencilRevision must equal ${expected.openPencilRevision}`)
  }
  const rawFiles = recordFrom(manifest.files)
  if (rawFiles === undefined) fail('files must be an object')
  const files = rawFiles as Record<string, unknown>
  for (const [path, digest] of Object.entries(files)) {
    if (!safeManifestPath(path)) fail(`files contains unsafe path ${JSON.stringify(path)}`)
    if (typeof digest !== 'string' || !SHA256_PATTERN.test(digest)) {
      fail(`files[${JSON.stringify(path)}] must be a lowercase SHA-256 digest`)
    }
  }
  return Object.freeze({
    schemaVersion: EDITOR_RUNTIME_SCHEMA_VERSION,
    platform: manifest.platform as string,
    packageName: manifest.packageName as string,
    openPencilVersion: manifest.openPencilVersion as string,
    openPencilRevision: manifest.openPencilRevision as string,
    files: Object.freeze({ ...(files as Record<string, string>) }),
  })
}

/** Stable selector used by both npm package names and npm/<platform> staging. */
export function editorRuntimePlatformKey(
  platform: string = process.platform,
  arch: string = process.arch,
): string {
  return `${platform}-${arch}`
}

/** The five files that make one atomic staged runtime minimally bootable. */
export function editorRuntimeRequiredFiles(
  binaryName: string,
  layout: EditorRuntimeLayout = 'canonical',
): readonly string[] {
  const webRoot = layout === 'canonical' ? 'bin/web-bundle' : 'web/pkg'
  const canvasKitRoot = layout === 'canonical' ? `${webRoot}/canvaskit` : 'web/canvaskit'
  return Object.freeze([
    `bin/${binaryName}`,
    `${webRoot}/op_host_web.js`,
    `${webRoot}/op_host_web_bg.wasm`,
    `${canvasKitRoot}/canvaskit.js`,
    `${canvasKitRoot}/canvaskit.wasm`,
  ])
}

function defaultProjectRoot(): string {
  return resolve(dirname(fileURLToPath(import.meta.url)), '..')
}

function readJson(path: string, io: RuntimeIo, kind: string): unknown {
  let bytes: Buffer
  try {
    bytes = io.readFile(path)
  } catch (error) {
    throw new EditorRuntimeUnavailableError(
      kind === 'catalog' ? 'catalog-unavailable' : 'invalid-runtime',
      `Unable to read OpenPencil ${kind} at ${path}`,
      [error instanceof Error ? error.message : String(error)],
      { cause: error },
    )
  }
  try {
    return JSON.parse(bytes.toString('utf8')) as unknown
  } catch (error) {
    throw new EditorRuntimeUnavailableError(
      kind === 'catalog' ? 'invalid-catalog' : 'invalid-runtime',
      `OpenPencil ${kind} at ${path} is not valid JSON`,
      [error instanceof Error ? error.message : String(error)],
      { cause: error },
    )
  }
}

function isFile(path: string, io: RuntimeIo): boolean {
  try {
    return io.stat(path).isFile()
  } catch {
    return false
  }
}

function isDirectory(path: string, io: RuntimeIo): boolean {
  try {
    return io.stat(path).isDirectory()
  } catch {
    return false
  }
}

function sha256(path: string, io: RuntimeIo): string {
  try {
    return createHash('sha256').update(io.readFile(path)).digest('hex')
  } catch (error) {
    throw new EditorRuntimeUnavailableError(
      'invalid-runtime',
      `Unable to hash OpenPencil runtime file ${path}`,
      [error instanceof Error ? error.message : String(error)],
      { cause: error },
    )
  }
}

interface RuntimeLayoutPaths {
  kind: EditorRuntimeLayout
  webBundleDir: string
  canvasKitDir: string
  requiredFiles: readonly string[]
}

function runtimeLayoutPaths(
  root: string,
  binaryName: string,
  kind: EditorRuntimeLayout,
): RuntimeLayoutPaths {
  const webBundleDir = kind === 'canonical'
    ? join(root, 'bin', 'web-bundle')
    : join(root, 'web', 'pkg')
  return {
    kind,
    webBundleDir,
    canvasKitDir: kind === 'canonical'
      ? join(webBundleDir, 'canvaskit')
      : join(root, 'web', 'canvaskit'),
    requiredFiles: editorRuntimeRequiredFiles(binaryName, kind),
  }
}

function layoutHasEvidence(
  layout: RuntimeLayoutPaths,
  manifest: EditorRuntimeManifest,
  io: RuntimeIo,
): boolean {
  if (isDirectory(layout.webBundleDir, io) || isDirectory(layout.canvasKitDir, io)) return true
  const assetPaths = new Set(layout.requiredFiles.slice(1))
  return Object.keys(manifest.files).some(path => assetPaths.has(path))
}

function layoutPayloadIsComplete(root: string, layout: RuntimeLayoutPaths, io: RuntimeIo): boolean {
  return (
    isDirectory(layout.webBundleDir, io)
    && isDirectory(layout.canvasKitDir, io)
    && layout.requiredFiles.slice(1).every(relativePath => (
      isFile(join(root, ...relativePath.split('/')), io)
    ))
  )
}

function selectRuntimeLayout(
  root: string,
  binaryName: string,
  manifest: EditorRuntimeManifest,
  io: RuntimeIo,
  forcedLayout?: EditorRuntimeLayout,
): RuntimeLayoutPaths {
  const canonical = runtimeLayoutPaths(root, binaryName, 'canonical')
  const legacy = runtimeLayoutPaths(root, binaryName, 'legacy')
  if (layoutPayloadIsComplete(root, canonical, io) && layoutPayloadIsComplete(root, legacy, io)) {
    throw new EditorRuntimeUnavailableError(
      'invalid-runtime',
      `OpenPencil runtime under ${root} contains both canonical and legacy web layouts; remove the legacy web directory so daemon discovery and injected paths cannot diverge`,
    )
  }
  const layout = forcedLayout === 'canonical'
    ? canonical
    : forcedLayout === 'legacy'
      ? legacy
      : layoutHasEvidence(canonical, manifest, io)
        ? canonical
        : layoutHasEvidence(legacy, manifest, io)
          ? legacy
          : undefined
  if (layout === undefined) {
    throw new EditorRuntimeUnavailableError(
      'invalid-runtime',
      `OpenPencil runtime under ${root} has neither the canonical bin/web-bundle layout nor the legacy web/{pkg,canvaskit} layout`,
    )
  }
  if (!isDirectory(layout.webBundleDir, io) || !isDirectory(layout.canvasKitDir, io)) {
    const description = layout.kind === 'canonical'
      ? 'bin/web-bundle with its canvaskit subdirectory'
      : 'web/pkg and web/canvaskit directories'
    throw new EditorRuntimeUnavailableError(
      'invalid-runtime',
      `OpenPencil runtime under ${root} must contain the complete ${description}`,
    )
  }
  return layout
}

function validateRuntimeRoot(
  root: string,
  source: EditorRuntimeSource,
  expected: RuntimeExpectation,
  io: RuntimeIo,
  forcedLayout?: EditorRuntimeLayout,
): EditorRuntime {
  const manifestPath = join(root, 'openpencil-runtime.json')
  if (!isFile(manifestPath, io)) {
    throw new EditorRuntimeUnavailableError(
      'invalid-runtime',
      `OpenPencil ${source} runtime is missing ${manifestPath}`,
    )
  }
  let manifest: EditorRuntimeManifest
  try {
    manifest = parseEditorRuntimeManifest(readJson(manifestPath, io, 'runtime manifest'), {
      platform: expected.platform.id,
      packageName: expected.platform.packageName,
      openPencilVersion: expected.version,
      openPencilRevision: expected.revision,
    })
  } catch (error) {
    if (error instanceof EditorRuntimeUnavailableError) {
      throw new EditorRuntimeUnavailableError(
        'invalid-runtime',
        `${error.message} (${manifestPath})`,
        error.diagnostics,
        { cause: error },
      )
    }
    throw error
  }

  const layout = selectRuntimeLayout(
    root,
    expected.platform.binaryName,
    manifest,
    io,
    forcedLayout,
  )
  for (const relativePath of layout.requiredFiles) {
    if (manifest.files[relativePath] === undefined) {
      throw new EditorRuntimeUnavailableError(
        'invalid-runtime',
        `OpenPencil runtime manifest ${manifestPath} is missing SHA-256 for ${relativePath}`,
      )
    }
  }
  for (const [relativePath, expectedDigest] of Object.entries(manifest.files)) {
    const absolutePath = join(root, ...relativePath.split('/'))
    if (!isFile(absolutePath, io)) {
      throw new EditorRuntimeUnavailableError(
        'invalid-runtime',
        `OpenPencil runtime file is missing or not regular: ${absolutePath}`,
      )
    }
    const actualDigest = sha256(absolutePath, io)
    if (actualDigest !== expectedDigest) {
      throw new EditorRuntimeUnavailableError(
        'invalid-runtime',
        `OpenPencil runtime SHA-256 mismatch for ${absolutePath}`,
        [`expected ${expectedDigest}`, `actual ${actualDigest}`],
      )
    }
  }

  const binary = join(root, 'bin', expected.platform.binaryName)
  if (expected.platform.os !== 'win32') {
    try {
      io.chmod(binary, 0o755)
    } catch {
      // Best effort, matching op-vscode. spawn will surface a real permissions
      // failure, while immutable package stores remain usable when already +x.
    }
  }
  return Object.freeze({
    binary,
    webBundleDir: layout.webBundleDir,
    canvasKitDir: layout.canvasKitDir,
    openPencilVersion: expected.version,
    revision: expected.revision,
    source,
  })
}

function overridePaths(
  env: Readonly<Record<string, string | undefined>>,
): { binary: string; webBundleDir: string; canvasKitDir: string } | undefined {
  const values = {
    binary: pluginEnv(EDITOR_RUNTIME_ENV_SUFFIX.binary, { env })?.trim(),
    webBundleDir: pluginEnv(EDITOR_RUNTIME_ENV_SUFFIX.webBundleDir, { env })?.trim(),
    canvasKitDir: pluginEnv(EDITOR_RUNTIME_ENV_SUFFIX.canvasKitDir, { env })?.trim(),
  }
  const present = Object.entries(values).filter(([, value]) => value !== undefined && value.length > 0)
  if (present.length === 0) return undefined
  if (present.length !== 3) {
    const missing = Object.entries(values)
      .filter(([, value]) => value === undefined || value.length === 0)
      .map(([key]) => EDITOR_RUNTIME_ENV[key as keyof typeof EDITOR_RUNTIME_ENV])
    throw new EditorRuntimeUnavailableError(
      'partial-override',
      `OpenPencil editor runtime override is partial; set all three variables together. Missing: ${missing.join(', ')}`,
    )
  }
  return {
    binary: resolve(values.binary as string),
    webBundleDir: resolve(values.webBundleDir as string),
    canvasKitDir: resolve(values.canvasKitDir as string),
  }
}

function overrideRoot(
  paths: { binary: string; webBundleDir: string; canvasKitDir: string },
  platform: EditorRuntimePlatform,
): { root: string; layout: EditorRuntimeLayout } {
  const root = dirname(dirname(paths.binary))
  const expectedBinary = join(root, 'bin', platform.binaryName)
  if (paths.binary === expectedBinary) {
    for (const layout of ['canonical', 'legacy'] as const) {
      const expected = runtimeLayoutPaths(root, platform.binaryName, layout)
      if (
        paths.webBundleDir === expected.webBundleDir
        && paths.canvasKitDir === expected.canvasKitDir
      ) return { root, layout }
    }
  }
  throw new EditorRuntimeUnavailableError(
    'invalid-override',
    'OpenPencil editor runtime overrides must identify one atomic runtime root using either bin/web-bundle/{canvaskit} or legacy web/{pkg,canvaskit}',
    [
      `binary: ${paths.binary}`,
      `web bundle: ${paths.webBundleDir}`,
      `CanvasKit: ${paths.canvasKitDir}`,
    ],
  )
}

function runtimeIo(options: ResolveEditorRuntimeOptions): RuntimeIo {
  return {
    readFile: options.readFile ?? (path => readFileSync(path)),
    stat: options.stat ?? (path => statSync(path)),
    chmod: options.chmod ?? ((path, mode) => chmodSync(path, mode)),
  }
}

/** Resolve and fully verify one atomic editor runtime, or throw diagnostics. */
export function resolveEditorRuntime(options: ResolveEditorRuntimeOptions = {}): EditorRuntime {
  const platformName = options.platform ?? process.platform
  const architecture = options.arch ?? process.arch
  const key = editorRuntimePlatformKey(platformName, architecture)
  const env = options.env ?? process.env
  // A partial explicit override is a configuration error even when the catalog
  // or optional dependency is unavailable.
  const override = overridePaths(env)
  const projectRoot = options.projectRoot === undefined
    ? defaultProjectRoot()
    : resolve(options.projectRoot)
  const io = runtimeIo(options)

  let rawCatalog: unknown
  if (options.catalog !== undefined) {
    rawCatalog = options.catalog
  } else {
    rawCatalog = readJson(join(projectRoot, 'platforms.json'), io, 'catalog')
  }
  const catalog = parseEditorRuntimeCatalog(rawCatalog)
  const platform = catalog.platforms.find(entry => entry.os === platformName && entry.cpu === architecture)
  if (platform === undefined) {
    throw new EditorRuntimeUnavailableError(
      'unsupported-platform',
      `No OpenPencil editor runtime is published for ${key}`,
      [`supported: ${catalog.platforms.map(entry => entry.id).join(', ')}`],
    )
  }
  const expected: RuntimeExpectation = {
    platform,
    version: catalog.openPencil.version,
    revision: catalog.openPencil.revision,
  }

  if (override !== undefined) {
    const selected = overrideRoot(override, platform)
    return validateRuntimeRoot(selected.root, 'override', expected, io, selected.layout)
  }

  const resolvePackageJson = options.resolvePackageJson
    ?? (specifier => requireFromPlugin.resolve(specifier))
  let packageJson: string | undefined
  let packageResolutionDiagnostic: string | undefined
  try {
    packageJson = resolvePackageJson(`${platform.packageName}/package.json`)
  } catch (error) {
    packageResolutionDiagnostic = error instanceof Error ? error.message : String(error)
  }
  const developmentRoot = join(projectRoot, 'npm', platform.id)
  if (packageJson !== undefined && resolve(dirname(packageJson)) !== resolve(developmentRoot)) {
    return validateRuntimeRoot(dirname(packageJson), 'optional-package', expected, io)
  }

  const developmentManifest = join(developmentRoot, 'openpencil-runtime.json')
  if (isFile(developmentManifest, io)) {
    return validateRuntimeRoot(developmentRoot, 'development-package', expected, io)
  }

  throw new EditorRuntimeUnavailableError(
    'runtime-not-found',
    `OpenPencil editor runtime ${platform.id} is unavailable`,
    [
      `optional package: ${platform.packageName}`,
      `development runtime: ${developmentRoot}`,
      ...(packageResolutionDiagnostic === undefined ? [] : [`package resolution: ${packageResolutionDiagnostic}`]),
    ],
  )
}

/**
 * Optional form for hosts where the editor is an add-on. Integrity and
 * configuration failures still throw; only an absent/unsupported runtime is
 * represented as undefined.
 */
export function tryResolveEditorRuntime(
  options: ResolveEditorRuntimeOptions = {},
): EditorRuntime | undefined {
  try {
    return resolveEditorRuntime(options)
  } catch (error) {
    if (
      error instanceof EditorRuntimeUnavailableError
      && (error.code === 'runtime-not-found' || error.code === 'unsupported-platform')
    ) return undefined
    throw error
  }
}
