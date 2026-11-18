import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { prepareCanvasBrowser, browserLaunchStatus } from '../plugins/openpencil/scripts/browser-launch.mjs';
import { canvasTools, CanvasController } from '../plugins/openpencil/scripts/canvas-controller.mjs';
import { createUiHarness, settle } from './ui-harness.mjs';

const panelUrl = 'http://127.0.0.1:1234/canvas/' + 'a'.repeat(43);
const fixture = () => ({ panel: { url: panelUrl }, session: { assertLive() {} } });

test('Preparing an in-app request only returns the existing canvas and a ready receipt', async () => {
  const controller = fixture();
  const launch = prepareCanvasBrowser(controller, { theme: 'dark' });
  assert.equal(launch.state, 'pending');
  assert.equal(launch.panelUrl, `${panelUrl}#theme=dark&launch=${launch.launchId}`);
  assert.deepEqual(prepareCanvasBrowser(controller, { theme: 'light' }), launch);
  assert.throws(() => browserLaunchStatus(controller, { launchId: 'foreign' }, true), /Unknown/);
  assert.equal(browserLaunchStatus(controller, { launchId: launch.launchId }).state, 'pending');
  assert.equal(browserLaunchStatus(controller, { launchId: launch.launchId }, true).state, 'connected');
  for (const name of ['canvas_prepare_browser', 'canvas_launch_status']) {
    assert.deepEqual(canvasTools.find(tool => tool.name === name)._meta.ui.visibility, ['app']);
  }
  assert.equal(canvasTools.some(tool => tool.name === 'canvas_launch_ready'), false);
  assert.equal(canvasTools.some(tool => tool.name === 'canvas_launch_browser'), false);
  const source = await readFile(new URL('../plugins/openpencil/scripts/browser-launch.mjs', import.meta.url), 'utf8');
  assert.doesNotMatch(source, /child_process|execFile|xdg-open|rundll32|codex:\/\//);
});

test('Untrusted arguments, a stopped session and expired receipts cannot confirm an opening', () => {
  const controller = fixture();
  for (const args of [{}, { theme: 'evil' }, { theme: 'dark', url: 'https://foreign.example' }]) {
    assert.throws(() => prepareCanvasBrowser(controller, args), /theme|URLs/);
  }
  const result = prepareCanvasBrowser(controller, { theme: 'light' });
  assert.ok(controller.browserLaunch.expiresAt > Date.now() + 100_000);
  controller.browserLaunch.expiresAt = 0;
  assert.equal(browserLaunchStatus(controller, { launchId: result.launchId }, true).state, 'unconfirmed');
  controller.session.assertLive = () => { throw new Error('Stopped'); };
  assert.throws(() => prepareCanvasBrowser(controller, { theme: 'dark' }), /Stopped/);
});

test('A panel acknowledges only its native editor ready event and preserves the requested theme', async () => {
  const launchId = 'b'.repeat(22);
  const panel = await createUiHarness('panel', { embedded: false, dark: false, hash: `#theme=dark&launch=${launchId}` });
  assert.equal(panel.theme(), 'dark');
  panel.editor({ type: 'op-bridge/listening' });
  assert.equal(panel.rpcCalls.length, 0);
  panel.editor({ type: 'op-bridge/ready', generation: 0, revision: 0 });
  panel.editor({ type: 'op-bridge/ready', generation: 0, revision: 0 });
  await settle();
  assert.deepEqual(panel.rpcCalls.map(item => JSON.parse(item.options.body)), [{ name: 'canvas_launch_ready', arguments: { launchId } }]);
});

test('Ready receipts are origin checked and panel HTTP cannot prepare an opening request', async t => {
  const controller = new CanvasController();
  t.after(() => controller.close());
  const panel = await controller.ensurePanel();
  controller.browserLaunch = { launchId: 'b'.repeat(22), state: 'pending', expiresAt: Date.now() + 120_000 };
  controller.session = { assertLive() {}, close: async () => {} };
  const body = JSON.stringify({ name: 'canvas_launch_ready', arguments: { launchId: 'b'.repeat(22) } });
  const foreign = await fetch(panel.url + '/rpc', { method: 'POST', headers: { Origin: 'https://foreign.example' }, body });
  assert.equal(foreign.status, 403);
  assert.equal(controller.browserLaunch.state, 'pending');
  for (const name of ['canvas_prepare_browser', 'canvas_launch_browser']) {
    const denied = await fetch(panel.url + '/rpc', { method: 'POST', body: JSON.stringify({ name, arguments: { theme: 'dark' } }) });
    assert.equal(denied.status, 403);
  }
  const ready = await fetch(panel.url + '/rpc', { method: 'POST', headers: { Origin: panel.origin }, body });
  assert.equal((await ready.json()).structuredContent.state, 'connected');
});
