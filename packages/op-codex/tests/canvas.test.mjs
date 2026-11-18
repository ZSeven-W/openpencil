import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { atomicSave, sourceBytes } from '../plugins/openpencil/scripts/editor-session.mjs';
import { sha256, resolveRuntime } from '../plugins/openpencil/scripts/editor-runtime.mjs';
import { CanvasController, CANVAS_URI } from '../plugins/openpencil/scripts/canvas-controller.mjs';
import { snapshotMatches, diskDocument } from '../plugins/openpencil/scripts/document-format.mjs';
import { createUiHarness, initializeApp, openPanel } from './ui-harness.mjs';

const document = name => JSON.stringify({ version: '0.8.5', children: [{ type: 'frame', id: 'one', name }] });
async function dir(t) { const path = await mkdtemp(join(tmpdir(), 'codex-canvas-')); t.after(() => rm(path, { recursive: true, force: true })); return path; }

test('Flushed disk snapshots match inline images and metadata without masking stale edits or free-form data', () => {
  const live = { document: { version: '0.8.5', children: [{ type: 'image', src: 'data:image/png;base64,test',
    fill: [{type:'image',url:'data:image/png;base64,test'}], states: {hover:{stroke:{fill:[{type:'image',url:'data:image/png;base64,test'}]}}},
    state: {type:'image',src:'op-image:one'}, children:[{type:'text',content:'op-image:one'}] }] },
    activePageIndex: 1, preserveAuthoredGeometry: true, scenario: null };
  const disk = structuredClone(diskDocument(live));
  disk.images = {one:'data:image/png;base64,test'};
  disk.imageThumbs = {one:'preview'};
  disk.children[0].src = disk.children[0].fill[0].url = disk.children[0].states.hover.stroke.fill[0].url = 'op-image:one';
  assert.equal(snapshotMatches(disk,live),true);
  disk.children[0].children[0].content = 'changed';
  assert.equal(snapshotMatches(disk,live),false);
  disk.children[0].children[0].content = 'op-image:one';
  disk.editorMeta.activePageIndex = 0;
  assert.equal(snapshotMatches(disk,live),false);
});

test('Save preserves external changes, validates documents and refuses existing Save As paths', async t => {
  const root = await dir(t), path = join(root, 'source.op');
  const initial = document('original');
  await writeFile(path, initial);
  const baseline = sha256(initial);
  await writeFile(path, document('external'));
  await assert.rejects(atomicSave(path, document('canvas'), baseline), /changed outside/);
  assert.equal(JSON.parse(await readFile(path, 'utf8')).children[0].name, 'external');
  await assert.rejects(atomicSave(path, document('canvas'), null), /EEXIST/);
  await assert.rejects(atomicSave(join(root, 'invalid.op'), '{}', null), /OpenPencil document/);
  const next = join(root, 'new.op');
  const saved = await atomicSave(next, document('saved'), null);
  assert.equal(saved, sha256(await sourceBytes(next)));
  const updated = await atomicSave(next, document('updated'), saved);
  assert.equal(updated, sha256(await readFile(next)));
});

test('Incomplete or cross-platform runtimes fail before spawning an editor', async t => {
  const root = await dir(t);
  await writeFile(join(root, 'openpencil-runtime.json'), JSON.stringify({ schemaVersion: 1, platform: 'other-arch', openPencilVersion: '0.8.5', files: {} }));
  await assert.rejects(resolveRuntime(root), /incompatible/);
  await writeFile(join(root, 'openpencil-runtime.json'), JSON.stringify({ schemaVersion: 1, platform: `${process.platform}-${process.arch}`, openPencilVersion: '0.8.5', files: {} }));
  await assert.rejects(resolveRuntime(root), /incomplete/);
});

test('A managed canvas routes all tools to the same daemon and protects save_document', async t => {
  const controller = new CanvasController({ port: 1 });
  t.after(() => controller.close());
  const calls = [];
  controller.session = { source: '/example.op', draft: '/recovery.op', version: '0.8.5', port: 1234, token: 'private-lifecycle-token',
    assertLive() {}, close: async () => {}, save: async args => { calls.push(['save', args]); return { filePath: args.filePath }; },
    forward: async message => { calls.push(['forward',message]); return { jsonrpc:'2.0',id:message.id,result:{ tools:[], content:[{type:'text',text:'native'}] } }; } };
  const call = (name,args={}) => controller.dispatch({jsonrpc:'2.0',id:1,method:'tools/call',params:{name,arguments:args}});
  const native = await call('batch_design', { script:'example' });
  assert.equal(native.result.content[0].text, 'native');
  assert.equal(calls[0][1].params.name, 'batch_design');
  await call('save_document', {filePath:'/example.op'});
  assert.equal(calls[1][0], 'save');
  const result = await call('save_document', {filePath:'/example.op',sourceFilePath:'/foreign.op'});
  assert.equal(result.result.isError, true);
  assert.equal(calls.length,2);
  const opened = await call('open_canvas');
  const browser = await call('canvas_open_browser');
  assert.equal(browser.result.structuredContent.panelUrl, opened.result.structuredContent.panelUrl);
  assert.ok(!JSON.stringify(opened.result.structuredContent).includes('private-lifecycle-token'));
  assert.equal(opened.result._meta.canvas.token, 'private-lifecycle-token');
  assert.equal((await call('open_canvas',{filePath:'/different.op'})).result.isError, true);
});

test('UI resources and canvas tools remain available with the standalone editor offline', async t => {
  const controller = new CanvasController({ port: 1 });
  t.after(() => controller.close());
  const list = await controller.dispatch({ jsonrpc:'2.0',id:1,method:'tools/list' });
  const open = list.result.tools.find(tool=>tool.name==='open_canvas');
  assert.equal(open._meta.ui.resourceUri,CANVAS_URI);
  assert.deepEqual(open._meta['openai/ui'].entrypoints,[{type:'thread'}]);
  const browser = list.result.tools.find(tool => tool.name === 'canvas_open_browser');
  assert.ok(browser, 'A model-callable opener must remain available independently of UI entrypoints');
  assert.equal(browser._meta, undefined);
  const resource = await controller.dispatch({jsonrpc:'2.0',id:2,method:'resources/read',params:{uri:CANVAS_URI}});
  const item = resource.result.contents[0];
  assert.equal(item.mimeType,'text/html;profile=mcp-app');
  assert.deepEqual(item._meta.ui.csp.frameDomains,[controller.panel.origin]);
  assert.ok(!controller.session, 'UI preloading must not switch or spawn a document');
  const blocked = await fetch(controller.panel.url + '/rpc', {method:'POST',headers:{Origin:'https://foreign.example','Content-Type':'application/json'},body:'{}'});
  assert.equal(blocked.status,403);
  assert.equal((await fetch(controller.panel.origin+'/canvas/not-a-capability')).status,404);
  assert.equal((await fetch(controller.panel.url+'/rpc',{method:'POST',body:JSON.stringify({name:'batch_design'})})).status,403);
});

test('Shared queue preserves ordering and a failed write is not replayed', async () => {
  const controller=new CanvasController(); const calls=[];
  const failed=controller.enqueue(async()=>{calls.push('write');throw new Error('ambiguous timeout');});
  const read=controller.enqueue(async()=>{calls.push('read');return 1;});
  await assert.rejects(failed,/timeout/); assert.equal(await read,1); assert.deepEqual(calls,['write','read']);
});


test('MCP Apps handshake opens the panel and accepts selection only from the exact frame', async () => {
  const ui = await createUiHarness('app');
  assert.equal(ui.hostMessages[0].data.method, 'ui/initialize');
  await initializeApp(ui, 'light');
  const open = ui.hostMessages.find(item => item.data.method === 'tools/call').data;
  assert.equal(open.params.name, 'open_canvas');
  ui.host({jsonrpc:'2.0',method:'ui/notifications/tool-result',params:{structuredContent:{panelUrl:'invalid'}}});
  assert.equal(ui.frame.hidden, true);
  const url = await openPanel(ui);
  assert.equal(ui.frame.src, url + '#theme=light');
  assert.equal(ui.frame.hidden, false);
  const count = ui.hostMessages.length;
  ui.window.emit('message', {source:ui.frame.contentWindow,origin:'https://foreign.example',data:{type:'codex-canvas/context'}});
  assert.equal(ui.hostMessages.length, count);
  ui.window.emit('message', {source:ui.frame.contentWindow,origin:'http://127.0.0.1:1234',data:{type:'codex-canvas/context',filePath:'/design.op',selection:['one']}});
  const update = ui.hostMessages.at(-1).data;
  assert.equal(update.method, 'ui/update-model-context');
  assert.match(update.params.content[0].text, /design.op/);
  ui.host({jsonrpc:'2.0',id:update.id,result:{}});
});
