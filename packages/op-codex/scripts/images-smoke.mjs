import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, writeFile, copyFile, unlink } from 'node:fs/promises';
import { isAbsolute, join } from 'node:path';
import { CanvasController } from '../plugins/openpencil/scripts/canvas-controller.mjs';
import { liveDocument } from '../plugins/openpencil/scripts/document-format.mjs';

const options = {};
for (let i = 2; i < process.argv.length; i += 2) options[process.argv[i].replace(/^--/, '')] = process.argv[i + 1];
if (![options.runtime, options.output].every(value => typeof value === 'string' && isAbsolute(value))) {
  throw new Error('Usage: npm run images-smoke -- --runtime /absolute/runtime --output /absolute/evidence');
}
await mkdir(options.output, { recursive: true });
const output = await mkdtemp(join(options.output, 'run-'));
process.env.OPENPENCIL_CODEX_STATE_DIR = join(output, 'state');
const filePath = join(output, 'images.op'), asset = join(output, 'local-asset.png');
const query = options.query || 'alpine mountain lake forest reflections landscape';
await copyFile(new URL('../plugins/openpencil/assets/icon.png', import.meta.url), asset);
const initial = { version: '0.8.5', children: [{ id: 'board', type: 'frame', name: 'Image acquisition test',
  width: 760, height: 360, fill: [{ type: 'solid', color: '#F3F4F6' }], children: [
    { id: 'local', type: 'image', name: 'Local portable asset', x: 24, y: 74, width: 240, height: 240,
      imagePrompt: 'test unresolved generation hint' },
    { id: 'stock', type: 'image', name: 'Search fallback', x: 296, y: 74, width: 440, height: 240,
      imagePrompt: 'an alpine mountain lake with forest reflections' },
    { id: 'title', type: 'text', name: 'Untouched title', content: 'Portable asset + stock fallback',
      x: 24, y: 20, width: 700, height: 38, fontFamily: 'Inter', fontSize: 26, fill: [{ type: 'solid', color: '#18181B' }] },
  ] }] };
await writeFile(filePath, JSON.stringify(initial), { flag: 'wx' });
const controllers = new Set();
let id = 0;
const report = { status: 'FAIL', output, filePath, generatedCalls: 0,
  assetSource: 'Existing plugin icon, not newly generated', checks: [] };
const find = (document, targetId) => {
  const stack = [...(document.children || [])];
  while (stack.length) { const node = stack.pop(); if (node.id === targetId) return node; stack.push(...(node.children || [])); }
};
async function open() {
  const controller = new CanvasController({ runtimeRoot: options.runtime }); controllers.add(controller);
  await controller.open({ filePath }); return controller;
}
async function tool(controller, name, args = {}) {
  assert.ok(controller.session, 'Use only this test-owned runtime');
  const reply = await controller.dispatch({ jsonrpc: '2.0', id: ++id, method: 'tools/call', params: { name, arguments: args } });
  if (reply.error || reply.result.isError) throw new Error(`${name}: ${reply.error?.message || reply.result.content?.[0]?.text}`);
  return reply.result;
}
async function capture(controller, name) {
  const result = await tool(controller, 'get_screenshot', { nodeId: 'board' });
  const image = result.content.find(item => item.type === 'image'); assert.ok(image?.data);
  await writeFile(join(output, name), Buffer.from(image.data, 'base64'));
}
try {
  let controller = await open();
  const before = await controller.session.document();
  const imported = (await tool(controller, 'canvas_import_image', { filePath: asset,
    nodeId: 'local', pageId: '0', expectedVersion: before.version })).structuredContent;
  const after = await controller.session.document();
  const image = find(after.document, 'local');
  assert.deepEqual(Buffer.from(image.src.split(',')[1], 'base64'), await readFile(asset));
  assert.ok(!image.imagePrompt && !image.imageSearchQuery);
  assert.deepEqual(find(after.document, 'title'), find(before.document, 'title'));
  assert.deepEqual(find(after.document, 'stock'), find(before.document, 'stock'));
  for (const key of ['id', 'x', 'y', 'width', 'height']) assert.equal(image[key], find(before.document, 'local')[key]);
  report.checks.push({ name: 'local-import-exact-bytes-and-preserved-neighbors', imported });
  // No new generation: exercise the real default/fallback stock pipeline.
  await tool(controller, 'update_node', { nodeId: 'stock', data: {
    imagePrompt: null, imageSearchQuery: query } });
  const search = await tool(controller, 'enrich_images', { root_ids: ['stock'], timeout_seconds: 45 });
  const counts = search.structuredContent || JSON.parse(search.content.find(item => item.type === 'text').text);
  await writeFile(join(output, 'search-result.json'), JSON.stringify(counts, null, 2));
  report.checks.push({ name: 'generate-hint-converted-to-scoped-stock-search', query, counts,
    passed: Number(counts.targets) === 1 && Number(counts.resolved) === 1 });
  const searched = await controller.session.document();
  assert.deepEqual(find(searched.document, 'local'), image);
  if (Number(counts.resolved) === 1) assert.ok(find(searched.document, 'stock').src?.length > 0);
  await capture(controller, 'before-save.png');
  await tool(controller, 'canvas_save', { filePath });
  const saved = liveDocument(JSON.parse(await readFile(filePath, 'utf8')));
  assert.equal(find(saved, 'local').src, image.src);
  assert.ok(!JSON.stringify(saved).includes(asset));
  await unlink(asset);
  await controller.close(); controllers.delete(controller);
  controller = await open();
  const reopened = await controller.session.document();
  assert.equal(find(liveDocument(reopened.document), 'local').src, image.src);
  assert.deepEqual(find(reopened.document, 'title'), find(before.document, 'title'));
  await capture(controller, 'reopened.png');
  report.checks.push({ name: 'save-reopen-after-original-asset-deleted', passed: true });
  assert.equal(Number(counts.targets), 1, 'Stock search must target exactly the requested slot');
  assert.equal(Number(counts.resolved), 1, 'Stock provider did not resolve the requested slot; import/reopen checks are reported separately');
  report.status = 'PASS';
} catch (error) { report.error = error.message; process.exitCode = 1; }
finally {
  for (const controller of controllers) await controller.close();
  await writeFile(join(output, 'report.json'), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report, null, 2));
}
