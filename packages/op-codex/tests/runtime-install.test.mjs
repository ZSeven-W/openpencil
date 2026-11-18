import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, readdir, writeFile, rm, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { gzipSync, gunzipSync } from 'node:zlib';
import { installRuntime, ensureRuntime } from '../plugins/openpencil/scripts/runtime-install.mjs';
import { packArchive, unpackArchive } from '../plugins/openpencil/scripts/runtime-archive.mjs';
import { resolveRuntime, sha256 } from '../plugins/openpencil/scripts/editor-runtime.mjs';

const platform = `${process.platform}-${process.arch}`;
async function temporary(t) {
  const dir = await mkdtemp(join(tmpdir(), 'openpencil-install-test-'));
  t.after(() => rm(dir, { recursive: true, force: true })); return dir;
}
function fixture(change = () => {}) {
  const files = new Map([
    [`bin/op-host-web-server${process.platform === 'win32' ? '.exe' : ''}`, Buffer.from('native-test-fixture')],
    ['bin/web-bundle/op_host_web.js', Buffer.from('editor-js')],
    ['bin/web-bundle/op_host_web_bg.wasm', Buffer.from('wasm')],
    ['bin/web-bundle/canvaskit/canvaskit.js', Buffer.from('canvaskit-js')],
    ['bin/web-bundle/canvaskit/canvaskit.wasm', Buffer.from('canvaskit-wasm')],
    ['bin/web-bundle/assets/fonts/LICENSE', Buffer.from('font-license')],
  ]);
  const manifest = { schemaVersion: 1, platform, openPencilVersion: '0.8.5', openPencilRevision: 'a'.repeat(40),
    files: Object.fromEntries([...files].map(([name, bytes]) => [name, sha256(bytes)])) };
  change(manifest, files);
  files.set('openpencil-runtime.json', Buffer.from(JSON.stringify(manifest)));
  const archive = packArchive(files);
  const catalog = { schemaVersion: 1, release: 'runtime-v0.8.5-test', openPencilVersion: '0.8.5', openPencilRevision: 'a'.repeat(40),
    platforms: { [platform]: { platform, asset: `openpencil-runtime-${platform}.tar.gz`, bytes: archive.length, sha256: sha256(archive) } } };
  return { archive, catalog };
}
function options(state, value = fixture()) {
  let downloads = 0;
  return { state, catalog: value.catalog, progress() {}, fetchImpl: async url => {
    downloads++; assert.match(url, /^https:\/\/github\.com\/ZSeven-W\/codex-openpencil\/releases\/download\/runtime-v/);
    return new Response(value.archive, { headers: { 'content-length': String(value.archive.length) } });
  }, count: () => downloads };
}

test('Clean state downloads, checks, configures and reuses a complete immutable runtime', async t => {
  const state = await temporary(t), opts = options(state);
  const runtime = await ensureRuntime(undefined, opts);
  assert.equal(runtime.version, '0.8.5'); assert.equal(opts.count(), 1);
  assert.deepEqual(JSON.parse(await readFile(join(state, 'runtime.json'))),
    { root: runtime.root, source: 'release', release: opts.catalog.release, platform });
  assert.equal((await resolveRuntime(runtime.root)).binary, runtime.binary);
  assert.equal((await ensureRuntime(undefined, opts)).root, runtime.root);
  assert.equal((await installRuntime(opts)).root, runtime.root);
  assert.equal(opts.count(), 1);
  assert.deepEqual((await readdir(join(state, 'runtimes'))).filter(n => n.startsWith('.')), []);
});

test('Concurrent first use shares one download and one verified runtime', async t => {
  const opts = options(await temporary(t));
  const results = await Promise.all([installRuntime(opts), installRuntime(opts), installRuntime(opts)]);
  assert.equal(new Set(results.map(r => r.root)).size, 1); assert.equal(opts.count(), 1);
});

test('Checksum, truncation, excess bytes, HTTP failures and declared sizes fail without configuring', async t => {
  const state = await temporary(t), value = fixture();
  const responses = [
    () => new Response(Buffer.alloc(value.archive.length)),
    () => new Response(value.archive.subarray(0, -1)),
    () => new Response(Buffer.concat([value.archive, Buffer.from('x')])),
    () => new Response('not found', { status: 404 }),
    () => new Response(value.archive, { headers: { 'content-length': '1' } }),
    () => { throw new Error('network interrupted'); },
  ];
  for (const fetchImpl of responses) {
    await assert.rejects(installRuntime({ ...options(state, value), fetchImpl }));
    await assert.rejects(readFile(join(state, 'runtime.json')), { code: 'ENOENT' });
    assert.deepEqual(await readdir(join(state, 'runtimes')), []);
  }
});

test('Unsupported platforms and mismatched release metadata stop acquisition', async t => {
  const state = await temporary(t), opts = options(state);
  await assert.rejects(installRuntime({ ...opts, platform: 'plan9-x64' }), /No published/);
  assert.equal(opts.count(), 0);
  const value = fixture(manifest => { manifest.openPencilRevision = 'b'.repeat(40); });
  await assert.rejects(installRuntime(options(state, value)), /pinned release/);
  await assert.rejects(readFile(join(state, 'runtime.json')), { code: 'ENOENT' });
});

test('Invalid explicit configuration and damaged cache remain visible and are never silently replaced', async t => {
  const state = await temporary(t), opts = options(state);
  await writeFile(join(state, 'runtime.json'), '{broken');
  await assert.rejects(ensureRuntime(undefined, opts), /Invalid runtime configuration/);
  await writeFile(join(state, 'runtime.json'), JSON.stringify({ root: join(state, 'missing') }));
  await assert.rejects(ensureRuntime(undefined, opts), { code: 'ENOENT' });
  assert.equal(opts.count(), 0);
  const runtime = await installRuntime(opts);
  await writeFile(runtime.binary, 'tampered');
  await assert.rejects(installRuntime(opts), /integrity/); assert.equal(opts.count(), 1);
  assert.equal(await readFile(runtime.binary, 'utf8'), 'tampered');
});

test('Payload hashes, missing assets and unlisted files are checked before configuration', async t => {
  const state = await temporary(t);
  for (const change of [
    (m, f) => f.set('bin/web-bundle/op_host_web.js', Buffer.from('tampered')),
    (m, f) => f.delete('bin/web-bundle/op_host_web_bg.wasm'),
    (m, f) => f.set('unlisted.js', Buffer.from('extra')),
  ]) {
    await assert.rejects(installRuntime(options(state, fixture(change))));
    await assert.rejects(readFile(join(state, 'runtime.json')), { code: 'ENOENT' });
    assert.deepEqual(await readdir(join(state, 'runtimes')), []);
  }
});

function alterArchive(archive, change) {
  const tar = Buffer.from(gunzipSync(archive)); change(tar);
  tar.fill(32, 148, 156);
  tar.write([...tar.subarray(0, 512)].reduce((a,b) => a+b,0).toString(8).padStart(6,'0') + '\0 ',148,8);
  return gzipSync(tar);
}
test('Archive reader rejects traversal, Windows paths, symlinks, invalid headers and duplicate entries', () => {
  const good = packArchive(new Map([['ok', Buffer.from('bytes')]]));
  assert.equal(unpackArchive(good).get('ok').toString(), 'bytes');
  for (const name of ['../escape', '/absolute', 'C:/drive', 'a\\b', 'a/../escape']) {
    const bad = alterArchive(good, tar => { tar.fill(0,0,100); tar.write(name,0,100); });
    assert.throws(() => unpackArchive(bad), /Unsafe/);
  }
  assert.throws(() => unpackArchive(alterArchive(good, tar => { tar[156] = 50; })), /regular files/);
  const raw = gunzipSync(good);
  assert.throws(() => unpackArchive(gzipSync(Buffer.concat([raw.subarray(0,1024), raw]))), /archive entry/);
  assert.throws(() => unpackArchive(gzipSync(raw.subarray(0, 600))), /archive/);
  assert.throws(() => packArchive(new Map([['../bad', Buffer.from('x')]])), /Unsafe/);
});

test('A custom valid runtime is preserved when checking readiness', async t => {
  const state = await temporary(t), customState = join(state, 'custom');
  await mkdir(customState);
  const custom = await installRuntime(options(customState));
  await writeFile(join(state, 'runtime.json'), JSON.stringify({ root: custom.root }));
  const opts = options(state);
  assert.equal((await ensureRuntime(undefined, opts)).root, custom.root);
  assert.equal(opts.count(), 0);
});

test('Managed release upgrades activate only after validation and retain the previous runtime', async t => {
  const state = await temporary(t), first = await installRuntime(options(state));
  const upgraded = fixture(m => { m.openPencilRevision = 'b'.repeat(40); });
  upgraded.catalog.openPencilRevision = 'b'.repeat(40); upgraded.catalog.release = 'runtime-v0.8.5-next';
  const configBefore = await readFile(join(state, 'runtime.json'), 'utf8');
  await assert.rejects(ensureRuntime(undefined, { ...options(state, upgraded), fetchImpl: async () => { throw Error('offline'); } }), /offline/);
  assert.equal(await readFile(join(state, 'runtime.json'), 'utf8'), configBefore);
  const next = await ensureRuntime(undefined, options(state, upgraded));
  assert.notEqual(next.root, first.root);
  assert.equal((await resolveRuntime(first.root)).version, '0.8.5');
  assert.equal(JSON.parse(await readFile(join(state, 'runtime.json'))).release, upgraded.catalog.release);
});

test('A cached package from the wrong revision is rejected before changing configuration', async t => {
  const state = await temporary(t), opts = options(state), runtime = await installRuntime(opts);
  const path = join(runtime.root, 'openpencil-runtime.json');
  const manifest = JSON.parse(await readFile(path)); manifest.openPencilRevision = 'b'.repeat(40);
  await writeFile(path, JSON.stringify(manifest));
  await assert.rejects(installRuntime(opts), /pinned release/);
  await assert.rejects(ensureRuntime(undefined, opts), /pinned release/);
  assert.equal(opts.count(), 1);
});

test('Runtime catalog repository field selects the release repository and rejects invalid values', async t => {
  const value = fixture();
  value.catalog.repository = 'ZSeven-W/openpencil';
  let seenUrl;
  const runtime = await installRuntime({
    state: await temporary(t),
    catalog: value.catalog,
    progress() {},
    fetchImpl: async url => {
      seenUrl = url;
      return new Response(value.archive, { headers: { 'content-length': String(value.archive.length) } });
    },
  });
  assert.equal(runtime.version, '0.8.5');
  assert.match(seenUrl, /^https:\/\/github\.com\/ZSeven-W\/openpencil\/releases\/download\/runtime-v/);
  const invalid = fixture();
  invalid.catalog.repository = 'https://evil.example/repo';
  await assert.rejects(installRuntime(options(await temporary(t), invalid)), /Invalid runtime release repository/);
});
