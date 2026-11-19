import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createUiHarness, initializeApp, openPanel, settle } from './ui-harness.mjs';
import { CANVAS_URI, CanvasController } from '../plugins/openpencil/scripts/canvas-controller.mjs';
import { liveDocument, snapshotMatches, diskDocument } from '../plugins/openpencil/scripts/document-format.mjs';
import { importCanvasImage } from '../plugins/openpencil/scripts/canvas-images.mjs';
import { prepareCanvasBrowser, browserLaunchStatus } from '../plugins/openpencil/scripts/browser-launch.mjs';
import { parsePort, identityOf, errorReply } from '../plugins/openpencil/scripts/transport.mjs';

test('Doubao MCP App embeds loopback canvas iframe directly when host frameDomains allows it', async () => {
  const ui = await createUiHarness('app', { dark: true });
  await initializeApp(ui, 'dark', {
    serverTools: {},
    updateModelContext: {},
    message: { text: {} },
    sandbox: { csp: { frameDomains: ['http://127.0.0.1:1234'] } },
  });
  const url = await openPanel(ui);
  assert.equal(ui.frame.hidden, false);
  assert.equal(ui.element('launcher').hidden, true);
  assert.equal(ui.frame.src, `${url}#theme=dark`);
  assert.equal(ui.theme(), 'dark');

  ui.host({ jsonrpc: '2.0', method: 'ui/notifications/host-context-changed', params: { theme: 'light' } });
  await settle();
  assert.equal(ui.theme(), 'light');
  assert.deepEqual(structuredClone(ui.frameMessages.at(-1)), {
    data: { type: 'openpencil-canvas/theme', colorScheme: 'light' },
    origin: 'http://127.0.0.1:1234',
  });
});

test('Doubao MCP App falls back to launcher when host CSP omits loopback frame origin', async () => {
  const ui = await createUiHarness('app');
  await initializeApp(ui, 'light', {
    serverTools: {},
    message: { text: {} },
    sandbox: { csp: { frameDomains: [] } },
  });
  await openPanel(ui);
  assert.equal(ui.frame.hidden, true);
  assert.equal(ui.element('launcher').hidden, false);
  assert.equal(ui.element('open-canvas').disabled, false);
});

test('Doubao MCP App forwards selection context and codegen handoff cleanly', async () => {
  const ui = await createUiHarness('app');
  await initializeApp(ui, 'light');
  await openPanel(ui);

  ui.window.emit('message', {
    source: ui.frame.contentWindow,
    origin: 'http://127.0.0.1:1234',
    data: { type: 'openpencil-canvas/context', filePath: '/design.op', selection: { count: 1, nodeIds: ['n1'] } },
  });
  await settle();
  const ctxReq = ui.hostMessages.find(m => m.data.method === 'ui/update-model-context')?.data;
  assert.ok(ctxReq);
  assert.match(ctxReq.params.content[0].text, /design\.op/);

  ui.window.emit('message', {
    source: ui.frame.contentWindow,
    origin: 'http://127.0.0.1:1234',
    data: { type: 'openpencil-canvas/codegen', requestId: 'req-1', prompt: 'Convert frame' },
  });
  await settle();
  const msgReq = ui.hostMessages.find(m => m.data.method === 'ui/message')?.data;
  assert.ok(msgReq);
  assert.equal(msgReq.params.content[0].text, 'Convert frame');
});

test('CanvasController exposes Doubao MCP App resource metadata and browser canvas tool', async () => {
  const controller = new CanvasController();
  const list = await controller.dispatch({ jsonrpc: '2.0', id: 1, method: 'resources/list' });
  assert.equal(list.result.resources[0].uri, CANVAS_URI);
  assert.equal(list.result.resources[0].mimeType, 'text/html;profile=mcp-app');

  const tools = await controller.dispatch({ jsonrpc: '2.0', id: 2, method: 'tools/list' });
  const openTool = tools.result.tools.find(t => t.name === 'open_canvas');
  assert.equal(openTool._meta.ui.resourceUri, CANVAS_URI);
  assert.ok(tools.result.tools.some(t => t.name === 'canvas_open_browser'));

  const noSession = await controller.dispatch({
    jsonrpc: '2.0',
    id: 3,
    method: 'tools/call',
    params: { name: 'canvas_status', arguments: {} },
  });
  assert.equal(noSession.result.isError, true);
  await controller.close();
});

test('document-format resolves structural op-image refs without altering free-form text', () => {
  const disk = {
    version: '0.8.6',
    images: { hash1: 'data:image/png;base64,AAAA' },
    children: [
      { id: 'img1', type: 'image', src: 'op-image:hash1' },
      { id: 'txt1', type: 'text', content: 'op-image:hash1' },
    ],
  };
  const live = {
    document: liveDocument(disk),
    activePageIndex: 0,
    preserveAuthoredGeometry: true,
    scenario: null,
  };
  assert.equal(live.document.children[0].src, 'data:image/png;base64,AAAA');
  assert.equal(live.document.children[1].content, 'op-image:hash1');
  assert.equal(snapshotMatches(diskDocument(live), live), true);
});

test('browser-launch prepares and verifies one-time ready receipts', () => {
  const fakeController = {
    session: { assertLive() {} },
    panel: { url: 'http://127.0.0.1:4321/canvas/' + 'a'.repeat(43) },
  };
  const prepared = prepareCanvasBrowser(fakeController, { theme: 'dark' });
  assert.equal(prepared.state, 'pending');
  assert.match(prepared.panelUrl, /#theme=dark&launch=/);
  const ready = browserLaunchStatus(fakeController, { launchId: prepared.launchId }, true);
  assert.equal(ready.state, 'connected');
  assert.throws(() => prepareCanvasBrowser(fakeController, { theme: 'neon' }), /light or dark/);
});

test('importCanvasImage validates raster magic bytes and rejects non-raster files', async (t) => {
  const dir = await mkdtemp(join(tmpdir(), 'op-doubao-img-'));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const badFile = join(dir, 'not-an-image.png');
  await writeFile(badFile, '<svg></svg>');
  const fakeSession = {
    async document() { return { version: 1, activePageIndex: 0, document: { children: [{ id: 'i1', type: 'image' }] } }; },
  };
  await assert.rejects(
    () => importCanvasImage(fakeSession, { filePath: badFile, nodeId: 'i1', pageId: '0', expectedVersion: 1 }),
    /PNG, JPEG or WebP/,
  );
});

test('transport helpers validate loopback ports, server identity, and JSON-RPC error envelopes', () => {
  assert.equal(parsePort('3000'), 3000);
  assert.throws(() => parsePort('0'), /integer from 1 to 65535/);
  assert.throws(() => parsePort('abc'), /integer from 1 to 65535/);
  assert.deepEqual(identityOf({ result: { server: 'openpencil-mcp', mode: 'live' } }), { server: 'openpencil-mcp', mode: 'live' });
  assert.throws(() => identityOf({ result: { server: 'other' } }), /not a Rust OpenPencil/);
  assert.deepEqual(errorReply(7, -32600, 'Bad'), { jsonrpc: '2.0', id: 7, error: { code: -32600, message: 'Bad' } });
});
