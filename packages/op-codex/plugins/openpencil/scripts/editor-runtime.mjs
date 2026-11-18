import { createHash } from 'node:crypto';
import { readFile, lstat, realpath } from 'node:fs/promises';
import { homedir } from 'node:os';
import { join, resolve, relative, isAbsolute } from 'node:path';

export const stateRoot = () => process.env.OPENPENCIL_CODEX_STATE_DIR || join(homedir(), '.codex-openpencil');
export const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

export async function resolveRuntime(root) {
  if (!root) {
    root = process.env.OPENPENCIL_EDITOR_RUNTIME_DIR;
    if (!root) {
      try { root = JSON.parse(await readFile(join(stateRoot(), 'runtime.json'), 'utf8')).root; }
      catch { /* Report one actionable error for missing or malformed configuration. */ }
    }
  }
  if (typeof root !== 'string' || !isAbsolute(root)) {
    throw new Error('Configure a complete Rust editor runtime with npm run configure-runtime -- /absolute/runtime/root.');
  }
  root = await realpath(root);
  const manifest = JSON.parse(await readFile(join(root, 'openpencil-runtime.json'), 'utf8'));
  if (manifest.schemaVersion !== 1 || manifest.platform !== `${process.platform}-${process.arch}` ||
      !/^\d+\.\d+\.\d+/.test(manifest.openPencilVersion) || !manifest.files) {
    throw new Error('Editor runtime manifest or platform is incompatible.');
  }
  const binaryName = process.platform === 'win32' ? 'op-host-web-server.exe' : 'op-host-web-server';
  const required = [`bin/${binaryName}`, 'bin/web-bundle/op_host_web.js',
    'bin/web-bundle/op_host_web_bg.wasm', 'bin/web-bundle/canvaskit/canvaskit.js',
    'bin/web-bundle/canvaskit/canvaskit.wasm'];
  // Never mix a daemon and web bundle from different releases or trust a partial package.
  for (const name of required) if (!manifest.files[name]) throw new Error(`Runtime is incomplete: ${name}`);
  for (const [name, hash] of Object.entries(manifest.files)) {
    const path = resolve(root, name);
    if (isAbsolute(name) || relative(root, path).startsWith('..') || !/^[a-f0-9]{64}$/.test(hash)) {
      throw new Error('Invalid editor runtime file manifest.');
    }
    if (relative(root, await realpath(path)).startsWith('..') || !(await lstat(path)).isFile() || sha256(await readFile(path)) !== hash) {
      throw new Error(`Editor runtime integrity check failed: ${name}`);
    }
  }
  return { root, version: manifest.openPencilVersion, binary: join(root, 'bin', binaryName),
    webBundle: join(root, 'bin/web-bundle'), canvasKit: join(root, 'bin/web-bundle/canvaskit') };
}
