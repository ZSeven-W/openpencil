import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, rm, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { importCanvasImage } from '../plugins/openpencil/scripts/canvas-images.mjs';
import { CanvasController, canvasTools } from '../plugins/openpencil/scripts/canvas-controller.mjs';

const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jx8sAAAAASUVORK5CYII=', 'base64');
async function fixture(t, bytes = png) {
  const dir = await mkdtemp(join(tmpdir(), 'codex-images-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const filePath = join(dir, 'asset with spaces.bin');
  await writeFile(filePath, bytes);
  return { filePath, nodeId: 'photo', pageId: 'page-a', expectedVersion: 7 };
}
const value = () => ({ version: 7, activePageIndex: 0, document: { pages: [
  { id: 'page-a', children: [{ id: 'card', type: 'frame', children: [
    { id: 'photo', type: 'image', x: 8, width: 120, height: 80, imagePrompt: 'paint a forest' },
    { id: 'title', type: 'text', content: 'Keep this copy' },
  ] }] }, { id: 'page-b', children: [{ id: 'other', type: 'image', src: 'keep.jpg' }] },
] } });
function session(live = value(), behavior) {
  const calls = [];
  return { live, calls, document: async () => structuredClone(live), forward: async message => {
    calls.push(message);
    if (behavior) return behavior(message);
    Object.assign(live.document.pages[0].children[0].children[0], message.params.arguments.data);
    live.version++;
    return { result: { content: [] } };
  } };
}

test('Local image import embeds exact bytes and preserves the target layout, sibling and other page', async t => {
  const args = await fixture(t), host = session();
  const before = structuredClone(host.live.document);
  const imported = await importCanvasImage(host, args);
  assert.equal(imported.embedded, true); assert.equal(imported.saved, false);
  assert.equal(imported.mimeType, 'image/png'); assert.equal(imported.bytes, png.length);
  assert.match(imported.sha256, /^[a-f0-9]{64}$/);
  assert.equal(host.calls.length, 1);
  const call = host.calls[0].params;
  assert.equal(call.name, 'update_node'); assert.equal(call.arguments.pageId, 'page-a');
  const image = host.live.document.pages[0].children[0].children[0];
  assert.deepEqual(Buffer.from(image.src.split(',')[1], 'base64'), png);
  before.pages[0].children[0].children[0] = { ...before.pages[0].children[0].children[0],
    src: image.src, imagePrompt: null, imageSearchQuery: null };
  assert.deepEqual(host.live.document, before);
  assert.ok(!JSON.stringify(imported).includes('base64'));
});

test('Import rejects stale versions, switched pages, missing targets and non-image nodes before writing', async t => {
  const args = await fixture(t);
  for (const patch of [{ expectedVersion: 6 }, { pageId: 'page-b' }, { nodeId: 'gone' }, { nodeId: 'title' }]) {
    const host = session();
    await assert.rejects(importCanvasImage(host, { ...args, ...patch }));
    assert.equal(host.calls.length, 0);
  }
  await assert.rejects(importCanvasImage(session(), { ...args, expectedVersion: undefined }), /expectedVersion/);
  await assert.rejects(importCanvasImage(session(), { ...args, filePath: 'relative.png' }), /absolute/);
});

test('Import rejects non-raster, empty and oversized assets without reaching the editor', async t => {
  for (const bytes of [Buffer.from('<svg xmlns="http://www.w3.org/2000/svg"/>'), Buffer.alloc(0), Buffer.alloc(8 * 1024 * 1024 + 1)]) {
    const args = await fixture(t, bytes), host = session();
    await assert.rejects(importCanvasImage(host, args));
    assert.equal(host.calls.length, 0);
  }
  if (process.platform !== 'win32') {
    const args = await fixture(t), link = args.filePath + '.png';
    await symlink(args.filePath, link);
    await assert.rejects(importCanvasImage(session(), { ...args, filePath: link }), /regular/);
  }
});

test('Import detects PNG/JPEG/WebP by bytes rather than an untrusted extension', async t => {
  const formats = [
    [Buffer.from([0xff, 0xd8, 0xff, 0xe0]), 'image/jpeg'],
    [Buffer.from('524946460c000000574542505650384c', 'hex'), 'image/webp'],
  ];
  for (const [bytes, mime] of formats) {
    const result = await importCanvasImage(session(), await fixture(t, bytes));
    assert.equal(result.mimeType, mime);
  }
});

test('Import refuses documents that would exceed the save limit before mutating', async t => {
  const args = await fixture(t), live = value();
  live.document.pages[0].children.push({ id: 'large', type: 'text', content: 'x'.repeat(32 * 1024 * 1024) });
  const host = session(live);
  await assert.rejects(importCanvasImage(host, args), /document limit/);
  assert.equal(host.calls.length, 0);
});

test('An ambiguous import failure is never retried and readback must confirm the image', async t => {
  const args = await fixture(t);
  for (const behavior of [() => { throw new Error('timeout after write'); }, () => ({ result: { isError: true } }),
    () => ({ result: { content: [] } })]) {
    const host = session(value(), behavior);
    await assert.rejects(importCanvasImage(host, args));
    assert.equal(host.calls.length, 1);
  }
});

test('Managed import is model-only and the browser panel cannot read arbitrary local files', async t => {
  const tool = canvasTools.find(tool => tool.name === 'canvas_import_image');
  assert.deepEqual(tool._meta.ui.visibility, ['model']);
  const controller = new CanvasController();
  t.after(() => controller.close());
  const offline = await controller.dispatch({ jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name: tool.name } });
  assert.equal(offline.result.isError, true);
  const panel = await controller.ensurePanel();
  const response = await fetch(panel.url + '/rpc', { method: 'POST', body: JSON.stringify({ name: tool.name }) });
  assert.equal(response.status, 403);
});
