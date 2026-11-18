import test from 'node:test';
import assert from 'node:assert/strict';
import { createUiHarness, initializeApp, openPanel } from './ui-harness.mjs';

const notification = theme => ({ jsonrpc: '2.0', method: 'ui/notifications/host-context-changed', params: { theme } });
const panelTheme = colorScheme => ({ type: 'codex-canvas/theme', colorScheme });

test('Host theme reaches the panel on first navigation and changes without reloading it', async () => {
  const app = await createUiHarness('app');
  await initializeApp(app, 'dark');
  const url = await openPanel(app);
  assert.equal(app.theme(), 'dark');
  assert.equal(app.frame.src, url + '#theme=dark');
  app.frame.emit('load');
  assert.equal(app.frameMessages.at(-1).data.colorScheme, 'dark');
  assert.equal(app.frameMessages.at(-1).origin, new URL(url).origin);
  app.host(notification('light'));
  assert.equal(app.theme(), 'light');
  assert.equal(app.frameMessages.at(-1).data.colorScheme, 'light');
  app.host(notification('dark'));
  assert.equal(app.theme(), 'dark');
  assert.equal(app.navigation.length, 1, 'Theme changes must not reload unsaved canvas content');
  app.host({ jsonrpc: '2.0', method: 'ui/notifications/host-context-changed', params: { locale: 'zh-CN' } });
  app.host(notification('invalid'));
  app.systemTheme(false);
  assert.equal(app.theme(), 'dark', 'Partial/invalid updates and OS changes cannot clear the host theme');
});

test('Wrapper accepts host context and panel-ready only from their expected windows', async () => {
  const app = await createUiHarness('app');
  await initializeApp(app, 'light');
  const url = await openPanel(app);
  app.window.emit('message', { source: {}, origin: 'https://host.example', data: notification('dark') });
  assert.equal(app.theme(), 'light');
  const before = app.frameMessages.length;
  app.window.emit('message', { source: {}, origin: new URL(url).origin, data: { type: 'codex-canvas/ready' } });
  app.window.emit('message', { source: app.frame.contentWindow, origin: 'https://foreign.example', data: { type: 'codex-canvas/ready' } });
  assert.equal(app.frameMessages.length, before);
  app.window.emit('message', { source: app.frame.contentWindow, origin: new URL(url).origin, data: { type: 'codex-canvas/ready' } });
  assert.equal(app.frameMessages.at(-1).data.colorScheme, 'light');
});

test('Host media preference drives top-level panel startup and live switching', async () => {
  const panel = await createUiHarness('panel', { dark: true, embedded: false });
  assert.equal(new URL(panel.frame.src).searchParams.get('chat'), 'host');
  assert.equal(panel.theme(), 'dark');
  assert.equal(new URL(panel.frame.src).searchParams.get('theme'), 'dark');
  panel.frame.emit('load');
  assert.equal(panel.editorMessages.at(-2).type, 'op-bridge/init');
  assert.equal(panel.editorMessages.at(-1).type, 'op-bridge/theme');
  assert.equal(panel.editorMessages.at(-1).colorScheme, 'dark');
  panel.editor({ type: 'op-bridge/ready', generation: 3, revision: 4 });
  panel.editor({ type: 'op-bridge/dirty-changed', generation: 3, revision: 5, dirty: true });
  panel.systemTheme(false);
  assert.equal(panel.theme(), 'light');
  assert.equal(panel.root.style.colorScheme, 'light');
  assert.equal(panel.editorMessages.at(-1).colorScheme, 'light');
  panel.systemTheme(true);
  assert.equal(panel.editorMessages.at(-1).colorScheme, 'dark');
  assert.equal(panel.navigation.length, 1);
  assert.equal(panel.state('dirty'), true);
  assert.equal(panel.state('revision'), 5);
  assert.equal(panel.rpcCalls.length, 0, 'Changing theme cannot save or mutate the document');
});

test('Embedded panel seeds from host theme and resends the latest theme after editor startup', async () => {
  const panel = await createUiHarness('panel', { hash: '#theme=dark' });
  assert.equal(panel.theme(), 'dark');
  assert.equal(new URL(panel.frame.src).searchParams.get('theme'), 'dark');
  assert.ok(panel.hostMessages.some(item => item.data.type === 'codex-canvas/ready'));
  panel.host(panelTheme('light'));
  panel.editor({ type: 'op-bridge/listening' });
  assert.equal(panel.editorMessages.at(-2).type, 'op-bridge/init');
  assert.equal(panel.editorMessages.at(-1).colorScheme, 'light');
  panel.editor({ type: 'op-bridge/ready', generation: 1, revision: 0 });
  assert.equal(panel.editorMessages.at(-1).colorScheme, 'light');
  panel.systemTheme(true);
  assert.equal(panel.theme(), 'light', 'The explicit host theme wins over browser media');
  assert.equal(panel.navigation.length, 1);
});

test('Panel rejects themes from foreign windows, editor frames and changed parent origins', async () => {
  const panel = await createUiHarness('panel');
  panel.host(panelTheme('light'));
  panel.window.emit('message', { source: {}, origin: 'https://host.example', data: panelTheme('dark') });
  panel.window.emit('message', { source: panel.frame.contentWindow, origin: 'http://127.0.0.1:5678', data: panelTheme('dark') });
  panel.host(panelTheme('dark'), 'https://foreign.example');
  panel.host(panelTheme('invalid'));
  assert.equal(panel.theme(), 'light');
  panel.host(panelTheme('dark'));
  assert.equal(panel.theme(), 'dark');
});

test('Hosts without a theme retain live media fallback, including dark initial appearance', async () => {
  const app = await createUiHarness('app', { dark: true });
  await initializeApp(app);
  await openPanel(app);
  assert.equal(app.theme(), 'dark');
  app.systemTheme(false);
  assert.equal(app.theme(), 'light');
  assert.equal(app.frameMessages.at(-1).data.colorScheme, 'light');
  assert.equal(app.navigation.length, 1);
});
