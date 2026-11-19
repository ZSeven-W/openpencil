import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  assertSupportedLibc,
  parseEditorRuntimeCatalog,
  parseEditorRuntimeManifest,
  resolveRuntime,
  sha256,
} from '../plugins/openpencil/scripts/editor-runtime.mjs';
import { configureRuntime, ensureRuntime } from '../plugins/openpencil/scripts/runtime-install.mjs';

async function createFakeRuntime(root, { tamper = false, omitCanvasKit = false } = {}) {
  const binaryName = process.platform === 'win32' ? 'op-host-web-server.exe' : 'op-host-web-server';
  const files = {
    [`bin/${binaryName}`]: Buffer.from('#!/bin/sh\necho fake\n'),
    'bin/web-bundle/op_host_web.js': Buffer.from('export default {};\n'),
    'bin/web-bundle/op_host_web_bg.wasm': Buffer.from([0, 0x61, 0x73, 0x6d]),
    'bin/web-bundle/canvaskit/canvaskit.js': Buffer.from('export default {};\n'),
    'bin/web-bundle/canvaskit/canvaskit.wasm': Buffer.from([0, 0x61, 0x73, 0x6d]),
  };
  if (omitCanvasKit) delete files['bin/web-bundle/canvaskit/canvaskit.wasm'];
  const digests = {};
  for (const [rel, content] of Object.entries(files)) {
    const abs = join(root, ...rel.split('/'));
    await mkdir(join(abs, '..'), { recursive: true });
    await writeFile(abs, content);
    digests[rel] = sha256(content);
  }
  if (tamper) {
    await writeFile(join(root, 'bin/web-bundle/op_host_web.js'), 'export default { tampered: true };\n');
  }
  await writeFile(
    join(root, 'openpencil-runtime.json'),
    JSON.stringify({
      schemaVersion: 1,
      platform: `${process.platform}-${process.arch}`,
      packageName: `@zseven-w/qwenwork-openpencil-${process.platform}-${process.arch}`,
      openPencilVersion: '0.8.6',
      openPencilRevision: 'a'.repeat(40),
      files: digests,
    }, null, 2),
  );
}

test('assertSupportedLibc enforces glibc floor for Linux targets', () => {
  assert.doesNotThrow(() => assertSupportedLibc('darwin-arm64', undefined));
  assert.doesNotThrow(() => assertSupportedLibc('linux-x64', '2.35'));
  assert.doesNotThrow(() => assertSupportedLibc('linux-arm64', '2.39'));
  assert.throws(() => assertSupportedLibc('linux-x64', ''), /glibc 2\.35\+/);
  assert.throws(() => assertSupportedLibc('linux-arm64', '2.35'), /glibc 2\.39\+/);
});

test('parseEditorRuntimeCatalog and parseEditorRuntimeManifest reject invalid entries and traversal paths', () => {
  assert.throws(() => parseEditorRuntimeCatalog(null), /root must be an object/);
  assert.throws(() => parseEditorRuntimeManifest({
    schemaVersion: 1,
    platform: 'darwin-arm64',
    packageName: '@zseven-w/qwenwork-openpencil-darwin-arm64',
    openPencilVersion: '0.8.6',
    openPencilRevision: 'a'.repeat(40),
    files: { '../escape': 'a'.repeat(64) },
  }, {
    platform: 'darwin-arm64',
    packageName: '@zseven-w/qwenwork-openpencil-darwin-arm64',
    openPencilVersion: '0.8.6',
    openPencilRevision: 'a'.repeat(40),
  }), /unsafe path/);
});

test('configureRuntime and ensureRuntime verify SHA-256 integrity and reject tampered or incomplete runtimes', async (t) => {
  const dir = await mkdtemp(join(tmpdir(), 'op-qw-runtime-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const goodRoot = join(dir, 'good');
  const badRoot = join(dir, 'tampered');
  const incompleteRoot = join(dir, 'incomplete');
  const state = join(dir, 'state');
  await createFakeRuntime(goodRoot);
  await createFakeRuntime(badRoot, { tamper: true });
  await createFakeRuntime(incompleteRoot, { omitCanvasKit: true });

  const configured = await configureRuntime(goodRoot, state, { source: 'custom' });
  assert.equal(configured.version, '0.8.6');
  const ensured = await ensureRuntime(undefined, { state, projectRoot: dir });
  assert.equal(ensured.root, configured.root);

  await assert.rejects(() => resolveRuntime(badRoot), /integrity check failed/);
  await assert.rejects(() => resolveRuntime(incompleteRoot), /Runtime is incomplete/);
});
