/**
 * Atomic OpenPencil editor runtime resolution.
 *
 * A runtime is always selected as one complete package root. The daemon and
 * both web asset directories are never assembled from unrelated installs.
 */
import { createHash } from 'node:crypto';
import { chmodSync, existsSync, lstatSync, readFileSync, realpathSync, statSync } from 'node:fs';
import { chmod, lstat, readFile, realpath } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { homedir } from 'node:os';
import { dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const EDITOR_RUNTIME_SCHEMA_VERSION = 1;

const requireFromPlugin = createRequire(import.meta.url);
const SHA256_PATTERN = /^[a-f0-9]{64}$/;
const REVISION_PATTERN = /^[a-f0-9]{40,64}$/;
const VERSION_PATTERN = /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/;
const SAFE_ID_PATTERN = /^[a-z0-9][a-z0-9-]*$/;

export const stateRoot = () => process.env.OPENPENCIL_CODEX_STATE_DIR || join(homedir(), '.codex-openpencil');
export const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

export class EditorRuntimeUnavailableError extends Error {
  constructor(code, message, diagnostics = [], options) {
    super(message, options);
    this.name = 'EditorRuntimeUnavailableError';
    this.code = code;
    this.diagnostics = diagnostics;
  }
}

export function assertSupportedLibc(platform, glibc) {
  if (!platform.startsWith('linux-')) return;
  const minor = platform === 'linux-arm64' ? 39 : 35;
  const [major, current] = String(glibc || '').split('.').map(Number);
  if (!major || major < 2 || (major === 2 && current < minor)) {
    throw new Error(`Published ${platform} runtimes require glibc 2.${minor}+; musl/Alpine and older systems need a custom runtime.`);
  }
}

export function editorRuntimePlatformKey(platform = process.platform, arch = process.arch) {
  return `${platform}-${arch}`;
}

export function editorRuntimeRequiredFiles(binaryName) {
  return Object.freeze([
    `bin/${binaryName}`,
    'bin/web-bundle/op_host_web.js',
    'bin/web-bundle/op_host_web_bg.wasm',
    'bin/web-bundle/canvaskit/canvaskit.js',
    'bin/web-bundle/canvaskit/canvaskit.wasm',
  ]);
}

export function defaultPackageRoot() {
  return resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
}

export function hasPackageCatalog(projectRoot = defaultPackageRoot()) {
  const path = join(resolve(projectRoot), 'platforms.json');
  try {
    return existsSync(path) && statSync(path).isFile();
  } catch {
    return false;
  }
}

function recordFrom(value) {
  return typeof value === 'object' && value !== null && !Array.isArray(value) ? value : undefined;
}

function nonEmptyString(value) {
  return typeof value === 'string' && value.trim() === value && value.length > 0;
}

function catalogError(message) {
  throw new EditorRuntimeUnavailableError('invalid-catalog', `Invalid OpenPencil runtime catalog: ${message}`);
}

/** Parse and validate platforms.json without performing any I/O. */
export function parseEditorRuntimeCatalog(value) {
  const catalog = recordFrom(value);
  if (catalog === undefined) catalogError('root must be an object');
  if (catalog.schemaVersion !== EDITOR_RUNTIME_SCHEMA_VERSION) {
    catalogError(`schemaVersion must be ${EDITOR_RUNTIME_SCHEMA_VERSION}`);
  }
  const openPencil = recordFrom(catalog.openPencil);
  if (openPencil === undefined) catalogError('openPencil must be an object');
  if (!nonEmptyString(openPencil.version) || !VERSION_PATTERN.test(openPencil.version)) {
    catalogError('openPencil.version must be a semantic version');
  }
  if (!nonEmptyString(openPencil.revision) || !REVISION_PATTERN.test(openPencil.revision)) {
    catalogError('openPencil.revision must be a full hexadecimal Git revision');
  }
  if (!Array.isArray(catalog.platforms) || catalog.platforms.length === 0) {
    catalogError('platforms must be a non-empty array');
  }
  const platforms = [];
  const ids = new Set();
  const selectors = new Set();
  const packageNames = new Set();
  for (const [index, rawPlatform] of catalog.platforms.entries()) {
    const entry = recordFrom(rawPlatform);
    if (entry === undefined) catalogError(`platforms[${index}] must be an object`);
    for (const field of ['id', 'os', 'cpu', 'rustTarget', 'runner', 'packageName', 'binaryName']) {
      if (!nonEmptyString(entry[field])) catalogError(`platforms[${index}].${field} must be a non-empty string`);
    }
    if (!SAFE_ID_PATTERN.test(entry.id)) catalogError(`platforms[${index}].id is invalid`);
    if (!SAFE_ID_PATTERN.test(entry.os)) catalogError(`platforms[${index}].os is invalid`);
    if (!SAFE_ID_PATTERN.test(entry.cpu)) catalogError(`platforms[${index}].cpu is invalid`);
    if (entry.libc !== undefined && !nonEmptyString(entry.libc)) {
      catalogError(`platforms[${index}].libc must be a non-empty string when present`);
    }
    const selector = `${entry.os}-${entry.cpu}`;
    if (entry.id !== selector) catalogError(`platforms[${index}].id must equal ${selector}`);
    const expectedPackageName = `@zseven-w/codex-openpencil-${entry.id}`;
    if (entry.packageName !== expectedPackageName) {
      catalogError(`platforms[${index}].packageName must equal ${expectedPackageName}`);
    }
    const expectedBinaryName = entry.os === 'win32' ? 'op-host-web-server.exe' : 'op-host-web-server';
    if (entry.binaryName !== expectedBinaryName) {
      catalogError(`platforms[${index}].binaryName must equal ${expectedBinaryName}`);
    }
    if (ids.has(entry.id)) catalogError(`duplicate platform id ${entry.id}`);
    if (selectors.has(selector)) catalogError(`duplicate platform selector ${selector}`);
    if (packageNames.has(entry.packageName)) catalogError(`duplicate packageName ${entry.packageName}`);
    ids.add(entry.id);
    selectors.add(selector);
    packageNames.add(entry.packageName);
    platforms.push(Object.freeze({ ...entry }));
  }
  return Object.freeze({
    schemaVersion: EDITOR_RUNTIME_SCHEMA_VERSION,
    openPencil: Object.freeze({ version: openPencil.version, revision: openPencil.revision }),
    platforms: Object.freeze(platforms),
  });
}

function safeManifestPath(path) {
  if (!nonEmptyString(path) || isAbsolute(path) || path.includes('\\')) return false;
  const segments = path.split('/');
  return segments.every(segment => segment !== '' && segment !== '.' && segment !== '..');
}

/** Parse one openpencil-runtime.json and bind it to its expected release. */
export function parseEditorRuntimeManifest(value, expected) {
  const fail = message => {
    throw new EditorRuntimeUnavailableError('invalid-runtime', `Invalid OpenPencil runtime manifest: ${message}`);
  };
  const manifest = recordFrom(value);
  if (manifest === undefined) fail('root must be an object');
  if (manifest.schemaVersion !== EDITOR_RUNTIME_SCHEMA_VERSION) {
    fail(`schemaVersion must be ${EDITOR_RUNTIME_SCHEMA_VERSION}`);
  }
  if (manifest.platform !== expected.platform) fail(`platform must equal ${expected.platform}`);
  if (expected.packageName !== undefined && manifest.packageName !== expected.packageName) {
    fail(`packageName must equal ${expected.packageName}`);
  }
  if (manifest.openPencilVersion !== expected.openPencilVersion) {
    fail(`openPencilVersion must equal ${expected.openPencilVersion}`);
  }
  if (manifest.openPencilRevision !== expected.openPencilRevision) {
    fail(`openPencilRevision must equal ${expected.openPencilRevision}`);
  }
  const files = recordFrom(manifest.files);
  if (files === undefined) fail('files must be an object');
  for (const [path, digest] of Object.entries(files)) {
    if (!safeManifestPath(path)) fail(`files contains unsafe path ${JSON.stringify(path)}`);
    if (typeof digest !== 'string' || !SHA256_PATTERN.test(digest)) {
      fail(`files[${JSON.stringify(path)}] must be a lowercase SHA-256 digest`);
    }
  }
  return Object.freeze({
    schemaVersion: EDITOR_RUNTIME_SCHEMA_VERSION,
    platform: manifest.platform,
    packageName: manifest.packageName,
    openPencilVersion: manifest.openPencilVersion,
    openPencilRevision: manifest.openPencilRevision,
    files: Object.freeze({ ...files }),
  });
}

function readJsonSync(path, io, kind) {
  let bytes;
  try {
    bytes = io.readFile(path);
  } catch (error) {
    throw new EditorRuntimeUnavailableError(
      kind === 'catalog' ? 'catalog-unavailable' : 'invalid-runtime',
      `Unable to read OpenPencil ${kind} at ${path}`,
      [error instanceof Error ? error.message : String(error)],
      { cause: error },
    );
  }
  try {
    return JSON.parse(bytes.toString('utf8'));
  } catch (error) {
    throw new EditorRuntimeUnavailableError(
      kind === 'catalog' ? 'invalid-catalog' : 'invalid-runtime',
      `OpenPencil ${kind} at ${path} is not valid JSON`,
      [error instanceof Error ? error.message : String(error)],
      { cause: error },
    );
  }
}

function isRegularFileSync(path, io) {
  try {
    const info = io.lstat(path);
    return info.isFile() && (!info.isSymbolicLink || !info.isSymbolicLink());
  } catch {
    return false;
  }
}

function isDirectorySync(path, io) {
  try {
    return io.stat(path).isDirectory();
  } catch {
    return false;
  }
}

function validateCatalogRuntimeRoot(rawRoot, source, expected, io) {
  let root;
  try {
    root = io.realpath(rawRoot);
  } catch (error) {
    throw new EditorRuntimeUnavailableError(
      'invalid-runtime',
      `OpenPencil ${source} runtime root does not exist: ${rawRoot}`,
      [error instanceof Error ? error.message : String(error)],
      { cause: error },
    );
  }
  const manifestPath = join(root, 'openpencil-runtime.json');
  if (!isRegularFileSync(manifestPath, io)) {
    throw new EditorRuntimeUnavailableError('invalid-runtime', `OpenPencil ${source} runtime is missing ${manifestPath}`);
  }
  let manifest;
  try {
    manifest = parseEditorRuntimeManifest(readJsonSync(manifestPath, io, 'runtime manifest'), {
      platform: expected.platform.id,
      packageName: expected.platform.packageName,
      openPencilVersion: expected.version,
      openPencilRevision: expected.revision,
    });
  } catch (error) {
    if (error instanceof EditorRuntimeUnavailableError) {
      throw new EditorRuntimeUnavailableError('invalid-runtime', `${error.message} (${manifestPath})`, error.diagnostics, { cause: error });
    }
    throw error;
  }
  const webBundleDir = join(root, 'bin', 'web-bundle');
  const canvasKitDir = join(webBundleDir, 'canvaskit');
  if (!isDirectorySync(webBundleDir, io) || !isDirectorySync(canvasKitDir, io)) {
    throw new EditorRuntimeUnavailableError(
      'invalid-runtime',
      `OpenPencil runtime under ${root} must contain bin/web-bundle with its canvaskit subdirectory`,
    );
  }
  for (const relativePath of editorRuntimeRequiredFiles(expected.platform.binaryName)) {
    if (manifest.files[relativePath] === undefined) {
      throw new EditorRuntimeUnavailableError(
        'invalid-runtime',
        `OpenPencil runtime manifest ${manifestPath} is missing SHA-256 for ${relativePath}`,
      );
    }
  }
  for (const [relativePath, expectedDigest] of Object.entries(manifest.files)) {
    const absolutePath = join(root, ...relativePath.split('/'));
    if (!isRegularFileSync(absolutePath, io)) {
      throw new EditorRuntimeUnavailableError('invalid-runtime', `OpenPencil runtime file is missing or not regular: ${absolutePath}`);
    }
    const resolvedPath = io.realpath(absolutePath);
    if (relative(root, resolvedPath).startsWith('..')) {
      throw new EditorRuntimeUnavailableError('invalid-runtime', `OpenPencil runtime file escapes package root: ${relativePath}`);
    }
    const actualDigest = sha256(io.readFile(absolutePath));
    if (actualDigest !== expectedDigest) {
      throw new EditorRuntimeUnavailableError(
        'invalid-runtime',
        `OpenPencil runtime SHA-256 mismatch for ${absolutePath}`,
        [`expected ${expectedDigest}`, `actual ${actualDigest}`],
      );
    }
  }
  const binary = join(root, 'bin', expected.platform.binaryName);
  if (expected.platform.os !== 'win32') {
    try {
      io.chmod(binary, 0o755);
    } catch {
      // Best effort; immutable package stores remain usable when already +x.
    }
  }
  return Object.freeze({
    binary,
    webBundleDir,
    canvasKitDir,
    openPencilVersion: expected.version,
    revision: expected.revision,
    source,
  });
}

function runtimeIo(options) {
  return {
    readFile: options.readFile ?? (path => readFileSync(path)),
    stat: options.stat ?? (path => statSync(path)),
    lstat: options.lstat ?? options.stat ?? (path => lstatSync(path)),
    realpath: options.realpath ?? (options.stat ? (path => resolve(path)) : (path => realpathSync.native(path))),
    chmod: options.chmod ?? ((path, mode) => chmodSync(path, mode)),
  };
}

/** Resolve and fully verify one atomic editor runtime from npm optionalDependencies or staged npm/<platform>. */
export function resolveEditorRuntime(options = {}) {
  const platformName = options.platform ?? process.platform;
  const architecture = options.arch ?? process.arch;
  const key = editorRuntimePlatformKey(platformName, architecture);
  const env = options.env ?? process.env;
  const projectRoot = options.projectRoot === undefined ? defaultPackageRoot() : resolve(options.projectRoot);
  const io = runtimeIo(options);

  const rawCatalog = options.catalog !== undefined
    ? options.catalog
    : readJsonSync(join(projectRoot, 'platforms.json'), io, 'catalog');
  const catalog = parseEditorRuntimeCatalog(rawCatalog);
  const platform = catalog.platforms.find(entry => entry.os === platformName && entry.cpu === architecture);
  if (platform === undefined) {
    throw new EditorRuntimeUnavailableError(
      'unsupported-platform',
      `No OpenPencil editor runtime is published for ${key}`,
      [`supported: ${catalog.platforms.map(entry => entry.id).join(', ')}`],
    );
  }
  const expected = {
    platform,
    version: catalog.openPencil.version,
    revision: catalog.openPencil.revision,
  };

  const explicitOverride = env.OPENPENCIL_EDITOR_RUNTIME_DIR?.trim();
  if (explicitOverride) {
    if (!isAbsolute(explicitOverride)) {
      throw new EditorRuntimeUnavailableError(
        'invalid-override',
        `OPENPENCIL_EDITOR_RUNTIME_DIR must be an absolute path: ${explicitOverride}`,
      );
    }
    return validateCatalogRuntimeRoot(explicitOverride, 'override', expected, io);
  }

  const resolvePackageJson = options.resolvePackageJson ?? (specifier => requireFromPlugin.resolve(specifier));
  let packageJson;
  let packageResolutionDiagnostic;
  try {
    packageJson = resolvePackageJson(`${platform.packageName}/package.json`);
  } catch (error) {
    packageResolutionDiagnostic = error instanceof Error ? error.message : String(error);
  }
  const developmentRoot = join(projectRoot, 'npm', platform.id);
  if (packageJson !== undefined && resolve(dirname(packageJson)) !== resolve(developmentRoot)) {
    return validateCatalogRuntimeRoot(dirname(packageJson), 'optional-package', expected, io);
  }

  const developmentManifest = join(developmentRoot, 'openpencil-runtime.json');
  if (isRegularFileSync(developmentManifest, io)) {
    return validateCatalogRuntimeRoot(developmentRoot, 'development-package', expected, io);
  }

  throw new EditorRuntimeUnavailableError(
    'runtime-not-found',
    `OpenPencil editor runtime ${platform.id} is unavailable`,
    [
      `optional package: ${platform.packageName}`,
      `development runtime: ${developmentRoot}`,
      ...(packageResolutionDiagnostic === undefined ? [] : [`package resolution: ${packageResolutionDiagnostic}`]),
    ],
  );
}

/** Optional form: throws on integrity/config errors, returns undefined only when absent or unsupported. */
export function tryResolveEditorRuntime(options = {}) {
  try {
    return resolveEditorRuntime(options);
  } catch (error) {
    if (error instanceof EditorRuntimeUnavailableError && (error.code === 'runtime-not-found' || error.code === 'unsupported-platform')) {
      return undefined;
    }
    throw error;
  }
}

/** Async validator for an explicit, custom, or configured runtime directory. */
export async function resolveRuntime(root, expected = {}) {
  if (!root) {
    root = process.env.OPENPENCIL_EDITOR_RUNTIME_DIR;
    if (!root) {
      try { root = JSON.parse(await readFile(join(stateRoot(), 'runtime.json'), 'utf8')).root; }
      catch { /* Report one actionable error for missing or malformed configuration. */ }
    }
  }
  if (typeof root !== 'string' || !isAbsolute(root)) {
    throw new Error('Configure a complete Rust editor runtime with codex-openpencil setup or codex-openpencil configure-runtime /absolute/runtime/root.');
  }
  root = await realpath(root);
  const manifest = JSON.parse(await readFile(join(root, 'openpencil-runtime.json'), 'utf8'));
  const currentPlatform = `${process.platform}-${process.arch}`;
  if (manifest.schemaVersion !== EDITOR_RUNTIME_SCHEMA_VERSION || manifest.platform !== currentPlatform ||
      !VERSION_PATTERN.test(manifest.openPencilVersion || '') || !recordFrom(manifest.files)) {
    throw new Error('Editor runtime manifest or platform is incompatible.');
  }
  if (expected.packageName !== undefined && manifest.packageName !== expected.packageName) {
    throw new Error('Editor runtime package does not match the configured release. Rerun codex-openpencil setup and reconnect the plugin.');
  }
  if (expected.openPencilVersion !== undefined && manifest.openPencilVersion !== expected.openPencilVersion) {
    throw new Error('Editor runtime version does not match the configured release. Rerun codex-openpencil setup and reconnect the plugin.');
  }
  if (expected.openPencilRevision !== undefined && manifest.openPencilRevision !== expected.openPencilRevision) {
    throw new Error('Editor runtime revision does not match the configured release. Rerun codex-openpencil setup and reconnect the plugin.');
  }
  const binaryName = process.platform === 'win32' ? 'op-host-web-server.exe' : 'op-host-web-server';
  const required = editorRuntimeRequiredFiles(binaryName);
  // Never mix a daemon and web bundle from different releases or trust a partial package.
  for (const name of required) if (!manifest.files[name]) throw new Error(`Runtime is incomplete: ${name}`);
  for (const [name, hash] of Object.entries(manifest.files)) {
    const path = resolve(root, name);
    if (!safeManifestPath(name) || relative(root, path).startsWith('..') || !SHA256_PATTERN.test(hash)) {
      throw new Error('Invalid editor runtime file manifest.');
    }
    const stat = await lstat(path);
    if (relative(root, await realpath(path)).startsWith('..') || !stat.isFile() || stat.isSymbolicLink() || sha256(await readFile(path)) !== hash) {
      throw new Error(`Editor runtime integrity check failed: ${name}`);
    }
  }
  const binary = join(root, 'bin', binaryName);
  if (process.platform !== 'win32') {
    try { await chmod(binary, 0o755); } catch { /* Immutable package stores may already be +x. */ }
  }
  return {
    root,
    version: manifest.openPencilVersion,
    revision: manifest.openPencilRevision,
    packageName: manifest.packageName,
    binary,
    webBundle: join(root, 'bin/web-bundle'),
    canvasKit: join(root, 'bin/web-bundle/canvaskit'),
  };
}
