import { mkdir, readFile, writeFile, rename, rm } from 'node:fs/promises';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { randomUUID } from 'node:crypto';
import { pathToFileURL } from 'node:url';
import {
  assertSupportedLibc,
  defaultPackageRoot,
  hasPackageCatalog,
  resolveEditorRuntime,
  resolveRuntime,
  stateRoot,
} from './editor-runtime.mjs';

export { assertSupportedLibc };

const MANAGED_SOURCES = new Set(['optional-package', 'development-package', 'release']);

export async function configureRuntime(root, state = stateRoot(), metadata = {}) {
  const runtime = await resolveRuntime(root);
  await mkdir(state, { recursive: true, mode: 0o700 });
  const temporary = join(state, `.runtime-${randomUUID()}.json`);
  try {
    await writeFile(temporary, JSON.stringify({ ...metadata, root: runtime.root }) + '\n', { flag: 'wx', mode: 0o600 });
    await rename(temporary, join(state, 'runtime.json'));
  } finally {
    await rm(temporary, { force: true });
  }
  return runtime;
}

/** Resolve the installed npm platform package (or staged dev package) and record it in private state. */
export async function activatePackageRuntime(options = {}) {
  const state = resolve(options.state ?? stateRoot());
  const [parsedOs, parsedArch] = typeof options.platform === 'string' && options.platform.includes('-')
    ? options.platform.split('-', 2)
    : [];
  const platformName = options.platformName ?? parsedOs ?? process.platform;
  const arch = options.arch ?? parsedArch ?? process.arch;
  const platform = options.platform ?? `${platformName}-${arch}`;
  if (platformName === 'linux') {
    const glibc = options.glibc !== undefined ? options.glibc : process.report?.getReport?.().header?.glibcVersionRuntime;
    assertSupportedLibc(platform, glibc);
  }
  const projectRoot = options.projectRoot ?? defaultPackageRoot();
  const resolved = resolveEditorRuntime({
    platform: platformName,
    arch,
    env: options.env ?? {},
    projectRoot,
    catalog: options.catalog,
    resolvePackageJson: options.resolvePackageJson,
    readFile: options.readFile,
    stat: options.stat,
    lstat: options.lstat,
    realpath: options.realpath,
    chmod: options.chmod,
  });
  const root = dirname(dirname(resolved.binary));
  const packageName = `@zseven-w/doubao-openpencil-${platform}`;
  const metadata = {
    source: resolved.source,
    packageName,
    openPencilVersion: resolved.openPencilVersion,
    openPencilRevision: resolved.revision,
    platform,
  };
  if (options.stat || options.readFile) {
    return {
      root,
      version: resolved.openPencilVersion,
      revision: resolved.revision,
      packageName,
      binary: resolved.binary,
      webBundle: resolved.webBundleDir,
      canvasKit: resolved.canvasKitDir,
      source: resolved.source,
    };
  }
  const runtime = await configureRuntime(root, state, metadata);
  return { ...runtime, source: resolved.source };
}

export const installRuntime = activatePackageRuntime;

export async function ensureRuntime(root, options = {}) {
  if (root || process.env.OPENPENCIL_EDITOR_RUNTIME_DIR) return resolveRuntime(root);
  const state = resolve(options.state ?? stateRoot());
  const projectRoot = options.projectRoot ?? defaultPackageRoot();
  const catalogAvailable = options.catalog !== undefined || hasPackageCatalog(projectRoot);

  let config;
  try {
    config = JSON.parse(await readFile(join(state, 'runtime.json'), 'utf8'));
  } catch (error) {
    if (error.code !== 'ENOENT') throw new Error(`Invalid runtime configuration: ${error.message}`);
    if (catalogAvailable) return activatePackageRuntime({ ...options, state, projectRoot });
    throw new Error(
      'OpenPencil editor runtime is not configured. Install @zseven-w/doubao-openpencil via npm (without --omit=optional) and run doubao-openpencil setup, or set OPENPENCIL_EDITOR_RUNTIME_DIR.',
    );
  }
  if (!config || typeof config !== 'object' || Array.isArray(config) || typeof config.root !== 'string' || !isAbsolute(config.root)) {
    throw new Error('Invalid runtime configuration: expected an absolute runtime root.');
  }
  if (MANAGED_SOURCES.has(config.source)) {
    if (catalogAvailable) return activatePackageRuntime({ ...options, state, projectRoot });
    const expected = config.source === 'release'
      ? {}
      : {
          packageName: config.packageName,
          openPencilVersion: config.openPencilVersion,
          openPencilRevision: config.openPencilRevision,
        };
    const runtime = await resolveRuntime(config.root, expected);
    return { ...runtime, source: config.source };
  }
  const runtime = await resolveRuntime(config.root);
  return { ...runtime, source: config.source || 'custom' };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    const runtime = await activatePackageRuntime();
    console.log(JSON.stringify({ ok: true, version: runtime.version, root: runtime.root, source: runtime.source }));
  } catch (error) {
    console.error(`OpenPencil runtime setup failed: ${error.message}`);
    process.exitCode = 1;
  }
}
