import { mkdir, mkdtemp, readFile, writeFile, rename, rm, lstat } from 'node:fs/promises';
import { join, resolve, isAbsolute } from 'node:path';
import { randomUUID } from 'node:crypto';
import { pathToFileURL } from 'node:url';
import { resolveRuntime, sha256, stateRoot } from './editor-runtime.mjs';
import { extractArchive } from './runtime-archive.mjs';
import { fetchRuntime } from './runtime-download.mjs';

export const catalogPath = new URL('../runtime-catalog.json', import.meta.url);
const inflight = new Map();

export function assertSupportedLibc(platform, glibc) {
  if (!platform.startsWith('linux-')) return;
  const minor = platform === 'linux-arm64' ? 39 : 35;
  const [major, current] = String(glibc || '').split('.').map(Number);
  if (!major || major < 2 || (major === 2 && current < minor)) {
    throw new Error(`Published ${platform} runtimes require glibc 2.${minor}+; musl/Alpine and older systems need a custom runtime.`);
  }
}

export async function configureRuntime(root, state = stateRoot(), metadata = {}) {
  const runtime = await resolveRuntime(root);
  await mkdir(state, { recursive: true, mode: 0o700 });
  const temporary = join(state, `.runtime-${randomUUID()}.json`);
  try {
    await writeFile(temporary, JSON.stringify({ ...metadata, root: runtime.root }) + '\n', { flag: 'wx', mode: 0o600 });
    await rename(temporary, join(state, 'runtime.json'));
  } finally { await rm(temporary, { force: true }); }
  return runtime;
}

async function download(asset, url, fetchImpl, progress) {
  const limit = 256 * 1024 * 1024;
  if (!Number.isSafeInteger(asset.bytes) || asset.bytes < 1 || asset.bytes > limit || !/^[a-f0-9]{64}$/.test(asset.sha256)) {
    throw new Error('Invalid published runtime checksum or size.');
  }
  const response = await fetchImpl(url, { signal: AbortSignal.timeout(300_000) });
  const discard = () => { if (response.body?.destroy) response.body.destroy(); else response.body?.cancel?.().catch(() => {}); };
  if (!response.ok || !response.body) {
    discard(); throw new Error(`Runtime download failed (HTTP ${response.status}). Run setup again after checking GitHub connectivity.`);
  }
  const declared = response.headers.get('content-length');
  if (declared && Number(declared) !== asset.bytes) {
    discard(); throw new Error('Runtime download size does not match the release catalog.');
  }
  const chunks = []; let size = 0, reported = 0;
  for await (const chunk of response.body) {
    size += chunk.length;
    if (size > asset.bytes) throw new Error('Runtime download exceeded its published size.');
    chunks.push(Buffer.from(chunk));
    if (size - reported > 8 * 1024 * 1024) { progress(`Downloaded ${Math.floor(size / 1024 / 1024)} MiB…`); reported = size; }
  }
  const bytes = Buffer.concat(chunks);
  if (size !== asset.bytes || sha256(bytes) !== asset.sha256) throw new Error('Runtime download integrity check failed. No runtime was installed.');
  return bytes;
}

export async function installRuntime(options = {}) {
  const state = resolve(options.state ?? stateRoot());
  const platform = options.platform ?? `${process.platform}-${process.arch}`;
  if (process.platform === 'linux') assertSupportedLibc(platform, process.report.getReport().header.glibcVersionRuntime);
  const catalog = options.catalog ?? JSON.parse(await readFile(catalogPath, 'utf8'));
  const entry = catalog.platforms?.[platform];
  if (catalog.schemaVersion !== 1 || !/^runtime-v[\w.-]+$/.test(catalog.release) ||
      !/^[a-f0-9]{40}$/.test(catalog.openPencilRevision) || !entry ||
      entry.platform !== platform || entry.asset !== `openpencil-runtime-${platform}.tar.gz`) {
    throw new Error(`No published OpenPencil runtime for ${platform}. See the repository's supported platforms; a complete custom runtime can still be configured.`);
  }
  const key = `${state}:${catalog.release}:${platform}`;
  if (inflight.has(key)) return inflight.get(key);
  const task = install({ ...options, state, platform, catalog, entry });
  inflight.set(key, task);
  try { return await task; } finally { inflight.delete(key); }
}

async function install({ state, platform, catalog, entry, fetchImpl = fetchRuntime, progress = message => process.stderr.write(`[OpenPencil] ${message}\n`) }) {
  const directory = join(state, 'runtimes');
  const destination = join(directory, `${catalog.release}-${platform}`);
  await mkdir(directory, { recursive: true, mode: 0o700 });
  let exists = false;
  try { await lstat(destination); exists = true; }
  catch (error) { if (error.code !== 'ENOENT') throw error; }
  // A damaged cache is reported; do not delete a user's runtime to hide it.
  const metadata = { source: 'release', release: catalog.release, platform };
  const validateRelease = async path => {
    const manifest = JSON.parse(await readFile(join(path, 'openpencil-runtime.json'), 'utf8'));
    if (manifest.openPencilRevision !== catalog.openPencilRevision || manifest.openPencilVersion !== catalog.openPencilVersion || manifest.platform !== platform) {
      throw new Error('Downloaded or cached runtime does not match the pinned release.');
    }
    await resolveRuntime(path);
    return manifest;
  };
  if (exists) { await validateRelease(destination); return configureRuntime(destination, state, metadata); }
  const staging = await mkdtemp(join(directory, '.install-'));
  try {
    progress(`Installing OpenPencil ${catalog.openPencilVersion} for ${platform}…`);
    const repository = catalog.repository ?? 'ZSeven-W/codex-openpencil';
    if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository)) {
      throw new Error('Invalid runtime release repository.');
    }
    const url = `https://github.com/${repository}/releases/download/${catalog.release}/${entry.asset}`;
    const names = await extractArchive(await download(entry, url, fetchImpl, progress), staging);
    const manifest = await validateRelease(staging);
    if (names.some(name => name !== 'openpencil-runtime.json' && !Object.hasOwn(manifest.files, name))) {
      throw new Error('Runtime archive contains files absent from its integrity manifest.');
    }
    try { await rename(staging, destination); }
    catch (error) {
      if (!['EEXIST', 'ENOTEMPTY', 'EPERM'].includes(error.code)) throw error;
      // Another process may have completed the same immutable runtime first.
      await validateRelease(destination);
    }
    const runtime = await configureRuntime(destination, state, metadata);
    progress('Runtime verified and configured.');
    return runtime;
  } finally { await rm(staging, { force: true, recursive: true }); }
}

export async function ensureRuntime(root, options = {}) {
  if (root || process.env.OPENPENCIL_EDITOR_RUNTIME_DIR) return resolveRuntime(root);
  const state = options.state ?? stateRoot();
  let config;
  try {
    config = JSON.parse(await readFile(join(state, 'runtime.json'), 'utf8'));
  } catch (error) {
    if (error.code !== 'ENOENT') throw new Error(`Invalid runtime configuration: ${error.message}`);
    return installRuntime({ ...options, state });
  }
  if (typeof config.root !== 'string' || !isAbsolute(config.root)) throw new Error('Invalid runtime configuration: expected an absolute runtime root.');
  if (config.source === 'release') {
    const catalog = options.catalog ?? JSON.parse(await readFile(catalogPath, 'utf8'));
    return installRuntime({ ...options, state, catalog });
  }
  return resolveRuntime(config.root);
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  try {
    const runtime = await installRuntime();
    console.log(JSON.stringify({ ok: true, version: runtime.version, root: runtime.root }));
  } catch (error) { console.error(`OpenPencil setup: ${error.message}`); process.exitCode = 1; }
}
