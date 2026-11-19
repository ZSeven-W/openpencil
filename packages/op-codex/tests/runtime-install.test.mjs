import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, realpath, writeFile, rm, mkdir, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  EditorRuntimeUnavailableError,
  parseEditorRuntimeCatalog,
  parseEditorRuntimeManifest,
  resolveEditorRuntime,
  tryResolveEditorRuntime,
  resolveRuntime,
  sha256,
} from '../plugins/openpencil/scripts/editor-runtime.mjs';
import {
  activatePackageRuntime,
  configureRuntime,
  ensureRuntime,
} from '../plugins/openpencil/scripts/runtime-install.mjs';

const osName = process.platform;
const arch = process.arch;
const platform = `${osName}-${arch}`;
const binaryName = osName === 'win32' ? 'op-host-web-server.exe' : 'op-host-web-server';
const packageName = `@zseven-w/codex-openpencil-${platform}`;

async function temporary(t) {
  const dir = await mkdtemp(join(tmpdir(), 'openpencil-runtime-test-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  return dir;
}

function makeCatalog(revision = 'a'.repeat(40), version = '0.8.6') {
  return {
    schemaVersion: 1,
    openPencil: { version, revision },
    platforms: [
      { id: 'darwin-arm64', os: 'darwin', cpu: 'arm64', rustTarget: 'aarch64-apple-darwin', runner: 'macos-15', packageName: '@zseven-w/codex-openpencil-darwin-arm64', binaryName: 'op-host-web-server' },
      { id: 'darwin-x64', os: 'darwin', cpu: 'x64', rustTarget: 'x86_64-apple-darwin', runner: 'macos-15-intel', packageName: '@zseven-w/codex-openpencil-darwin-x64', binaryName: 'op-host-web-server' },
      { id: 'linux-arm64', os: 'linux', cpu: 'arm64', libc: 'glibc', rustTarget: 'aarch64-unknown-linux-gnu', runner: 'ubuntu-24.04-arm', packageName: '@zseven-w/codex-openpencil-linux-arm64', binaryName: 'op-host-web-server' },
      { id: 'linux-x64', os: 'linux', cpu: 'x64', libc: 'glibc', rustTarget: 'x86_64-unknown-linux-gnu', runner: 'ubuntu-22.04', packageName: '@zseven-w/codex-openpencil-linux-x64', binaryName: 'op-host-web-server' },
      { id: 'win32-arm64', os: 'win32', cpu: 'arm64', rustTarget: 'aarch64-pc-windows-msvc', runner: 'windows-11-arm', packageName: '@zseven-w/codex-openpencil-win32-arm64', binaryName: 'op-host-web-server.exe' },
      { id: 'win32-x64', os: 'win32', cpu: 'x64', rustTarget: 'x86_64-pc-windows-msvc', runner: 'windows-2025', packageName: '@zseven-w/codex-openpencil-win32-x64', binaryName: 'op-host-web-server.exe' },
    ],
  };
}

async function writeRuntimeFixture(root, { revision = 'a'.repeat(40), version = '0.8.6', pkgName = packageName, mutate = () => {} } = {}) {
  const files = new Map([
    [`bin/${binaryName}`, Buffer.from('native-test-fixture')],
    ['bin/web-bundle/op_host_web.js', Buffer.from('editor-js')],
    ['bin/web-bundle/op_host_web_bg.wasm', Buffer.from('wasm')],
    ['bin/web-bundle/canvaskit/canvaskit.js', Buffer.from('canvaskit-js')],
    ['bin/web-bundle/canvaskit/canvaskit.wasm', Buffer.from('canvaskit-wasm')],
    ['bin/web-bundle/assets/iconify-catalog-core.json', Buffer.from('{}')],
  ]);
  const manifest = {
    schemaVersion: 1,
    platform,
    packageName: pkgName,
    openPencilVersion: version,
    openPencilRevision: revision,
    files: Object.fromEntries([...files].map(([name, bytes]) => [name, sha256(bytes)])),
  };
  mutate(manifest, files);
  for (const [rel, bytes] of files) {
    const dest = join(root, ...rel.split('/'));
    await mkdir(join(dest, '..'), { recursive: true });
    await writeFile(dest, bytes);
  }
  await writeFile(join(root, 'package.json'), JSON.stringify({ name: pkgName, version: '0.3.13' }) + '\n');
  await writeFile(join(root, 'openpencil-runtime.json'), JSON.stringify(manifest, null, 2) + '\n');
  return root;
}

test('Catalog and manifest parsers reject invalid versions, package names, paths and hashes', () => {
  const valid = makeCatalog();
  assert.equal(parseEditorRuntimeCatalog(valid).platforms.length, 6);
  const badPkg = structuredClone(valid);
  badPkg.platforms[0].packageName = '@zseven-w/dsh-openpencil-darwin-arm64';
  assert.throws(() => parseEditorRuntimeCatalog(badPkg), /packageName must equal/);

  const expected = { platform, packageName, openPencilVersion: '0.8.6', openPencilRevision: 'a'.repeat(40) };
  assert.throws(
    () => parseEditorRuntimeManifest({ schemaVersion: 1, platform, packageName, openPencilVersion: '0.8.6', openPencilRevision: 'a'.repeat(40), files: { '../escape': 'a'.repeat(64) } }, expected),
    /unsafe path/,
  );
  assert.throws(
    () => parseEditorRuntimeManifest({ schemaVersion: 1, platform, packageName, openPencilVersion: '0.8.6', openPencilRevision: 'a'.repeat(40), files: { 'bin/file': 'not-hex' } }, expected),
    /SHA-256/,
  );
});

test('resolveEditorRuntime prefers optional package, falls back to staged dev package, and validates SHA-256', async t => {
  const dir = await temporary(t);
  const projectRoot = join(dir, 'pkg');
  const optionalRoot = join(dir, 'node_modules', '@zseven-w', `codex-openpencil-${platform}`);
  const devRoot = join(projectRoot, 'npm', platform);
  await mkdir(projectRoot, { recursive: true });
  await writeFile(join(projectRoot, 'platforms.json'), JSON.stringify(makeCatalog()) + '\n');

  // Absent runtime throws runtime-not-found, while tryResolveEditorRuntime returns undefined.
  const missingResolve = () => { throw new Error('MODULE_NOT_FOUND'); };
  assert.throws(
    () => resolveEditorRuntime({ projectRoot, resolvePackageJson: missingResolve, env: {} }),
    error => error instanceof EditorRuntimeUnavailableError && error.code === 'runtime-not-found',
  );
  assert.equal(tryResolveEditorRuntime({ projectRoot, resolvePackageJson: missingResolve, env: {} }), undefined);
  assert.equal(tryResolveEditorRuntime({ projectRoot, platform: 'plan9', arch: 'x64', env: {} }), undefined);

  // Staged development package resolves when optional dependency is absent.
  await writeRuntimeFixture(devRoot);
  const devRuntime = resolveEditorRuntime({ projectRoot, resolvePackageJson: missingResolve, env: {} });
  assert.equal(devRuntime.source, 'development-package');
  assert.equal(devRuntime.binary, join(await realpath(devRoot), 'bin', binaryName));

  // Installed optional package takes precedence over development package.
  await writeRuntimeFixture(optionalRoot);
  const optRuntime = resolveEditorRuntime({
    projectRoot,
    resolvePackageJson: specifier => {
      assert.equal(specifier, `${packageName}/package.json`);
      return join(optionalRoot, 'package.json');
    },
    env: {},
  });
  assert.equal(optRuntime.source, 'optional-package');
  assert.equal(optRuntime.binary, join(await realpath(optionalRoot), 'bin', binaryName));

  // Tampering fails both resolveEditorRuntime and tryResolveEditorRuntime.
  await writeFile(join(optionalRoot, 'bin', 'web-bundle', 'op_host_web.js'), 'tampered');
  assert.throws(
    () => tryResolveEditorRuntime({ projectRoot, resolvePackageJson: () => join(optionalRoot, 'package.json'), env: {} }),
    error => error instanceof EditorRuntimeUnavailableError && error.code === 'invalid-runtime',
  );
});

test('Clean state activates optional package, persists runtime.json, and works from isolated plugin cache', async t => {
  const dir = await temporary(t);
  const state = join(dir, 'state');
  const projectRoot = join(dir, 'pkg');
  const isolatedCacheRoot = join(dir, 'codex-cache-no-catalog');
  const optionalRoot = join(dir, 'node_modules', '@zseven-w', `codex-openpencil-${platform}`);
  await mkdir(projectRoot, { recursive: true });
  await mkdir(isolatedCacheRoot, { recursive: true });
  await writeFile(join(projectRoot, 'platforms.json'), JSON.stringify(makeCatalog()) + '\n');
  await writeRuntimeFixture(optionalRoot);

  const opts = {
    state,
    projectRoot,
    resolvePackageJson: () => join(optionalRoot, 'package.json'),
    env: {},
  };
  const runtime = await ensureRuntime(undefined, opts);
  assert.equal(runtime.version, '0.8.6');
  assert.equal(runtime.source, 'optional-package');
  const saved = JSON.parse(await readFile(join(state, 'runtime.json'), 'utf8'));
  assert.equal(saved.source, 'optional-package');
  assert.equal(saved.packageName, packageName);
  assert.equal(saved.openPencilVersion, '0.8.6');
  assert.equal(saved.openPencilRevision, 'a'.repeat(40));

  // Isolated Codex plugin cache (no platforms.json) validates recorded runtime.json and its SHA-256s.
  const cached = await ensureRuntime(undefined, { state, projectRoot: isolatedCacheRoot, env: {} });
  assert.equal(cached.root, runtime.root);
  assert.equal(cached.version, '0.8.6');

  // If the package on disk changes revision without rerunning setup, isolated cache detects drift.
  await writeRuntimeFixture(optionalRoot, { revision: 'b'.repeat(40) });
  await assert.rejects(
    ensureRuntime(undefined, { state, projectRoot: isolatedCacheRoot, env: {} }),
    /revision does not match/,
  );

  // Without runtime.json or platforms.json, isolated cache fails with actionable install instructions.
  await rm(join(state, 'runtime.json'));
  await assert.rejects(
    ensureRuntime(undefined, { state, projectRoot: isolatedCacheRoot, env: {} }),
    /codex-openpencil setup/,
  );
});

test('Custom valid runtime is preserved, broken config fails loudly, and --repair-runtime re-selects package', async t => {
  const dir = await temporary(t);
  const state = join(dir, 'state');
  const projectRoot = join(dir, 'pkg');
  const optionalRoot = join(dir, 'optional');
  const customRoot = join(dir, 'custom');
  await mkdir(projectRoot, { recursive: true });
  await writeFile(join(projectRoot, 'platforms.json'), JSON.stringify(makeCatalog()) + '\n');
  await writeRuntimeFixture(optionalRoot);
  await writeRuntimeFixture(customRoot, { revision: 'c'.repeat(40) });

  await configureRuntime(customRoot, state, { source: 'custom' });
  const opts = { state, projectRoot, resolvePackageJson: () => join(optionalRoot, 'package.json'), env: {} };
  const preserved = await ensureRuntime(undefined, opts);
  assert.equal(preserved.root, (await resolveRuntime(customRoot)).root);
  assert.equal(preserved.source, 'custom');

  // Repair explicitly switches back to the installed npm platform package.
  const repaired = await activatePackageRuntime(opts);
  assert.equal(repaired.root, (await resolveRuntime(optionalRoot)).root);
  assert.equal(repaired.source, 'optional-package');

  // Malformed or dangling runtime.json is never silently overwritten.
  await writeFile(join(state, 'runtime.json'), '{broken');
  await assert.rejects(ensureRuntime(undefined, opts), /Invalid runtime configuration/);
  await writeFile(join(state, 'runtime.json'), JSON.stringify({ root: join(dir, 'missing') }));
  await assert.rejects(ensureRuntime(undefined, opts), { code: 'ENOENT' });
});

test('Managed upgrades and legacy release configs switch to the new verified npm package', async t => {
  const dir = await temporary(t);
  const state = join(dir, 'state');
  const projectRoot = join(dir, 'pkg');
  const v1Root = join(dir, 'v1');
  const v2Root = join(dir, 'v2');
  await mkdir(projectRoot, { recursive: true });
  await writeRuntimeFixture(v1Root, { revision: 'a'.repeat(40) });
  await writeRuntimeFixture(v2Root, { revision: 'b'.repeat(40) });

  // Simulate legacy GitHub Release runtime.json entry.
  await configureRuntime(v1Root, state, { source: 'release', release: 'runtime-v0.8.5-old', platform });

  // New package pins revision b... and resolves v2Root.
  await writeFile(join(projectRoot, 'platforms.json'), JSON.stringify(makeCatalog('b'.repeat(40))) + '\n');
  const upgraded = await ensureRuntime(undefined, {
    state,
    projectRoot,
    resolvePackageJson: () => join(v2Root, 'package.json'),
    env: {},
  });
  assert.equal(upgraded.revision, 'b'.repeat(40));
  assert.equal(upgraded.source, 'optional-package');
  assert.equal(JSON.parse(await readFile(join(state, 'runtime.json'), 'utf8')).source, 'optional-package');
});

test('Symlinks inside a runtime package are rejected', async t => {
  if (process.platform === 'win32') return;
  const dir = await temporary(t);
  const root = join(dir, 'runtime');
  const outside = join(dir, 'outside.js');
  await writeFile(outside, 'editor-js');
  await writeRuntimeFixture(root);
  const target = join(root, 'bin', 'web-bundle', 'op_host_web.js');
  await rm(target);
  await symlink(outside, target);
  await assert.rejects(resolveRuntime(root), /integrity check failed/);
});
