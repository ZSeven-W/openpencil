import test from 'node:test';
import assert from 'node:assert/strict';
import { createUiHarness, initializeApp, openPanel, settle } from './ui-harness.mjs';

const capabilities = { serverTools: {}, message: { text: {} } };
const launchId = 'b'.repeat(22);
const click = app => app.element('open-canvas').emit('click', { preventDefault() {} });
const calls = (app, name) => app.hostMessages.filter(item => item.data.params?.name === name).map(item => item.data);
const messages = app => app.hostMessages.filter(item => item.data.method === 'ui/message').map(item => item.data);
async function fixture() {
  const app = await createUiHarness('app', { protocol: 'codex-sandbox:' });
  await initializeApp(app, 'dark', capabilities);
  const url = await openPanel(app);
  return { app, url };
}
async function prepare(app, url) {
  const request = calls(app, 'canvas_prepare_browser').at(-1);
  app.host({ jsonrpc: '2.0', id: request.id, result: { structuredContent: {
    launchId, state: 'pending', panelUrl: `${url}#theme=dark&launch=${launchId}`,
  } } });
  await settle();
}

test('Codex sandbox exposes an in-app request without navigating a blocked iframe', async () => {
  const { app, url } = await fixture();
  assert.equal(app.navigation.length, 0);
  assert.equal(app.frame.hidden, true);
  assert.equal(app.element('launcher').hidden, false);
  assert.ok(app.element('canvas-url').value.includes(url + '#theme=dark'));
  assert.match(app.element('canvas-url').value, /Codex 内置浏览器/);
  app.host({ jsonrpc: '2.0', method: 'ui/notifications/host-context-changed', params: { theme: 'light' } });
  assert.ok(app.element('canvas-url').value.includes(url + '#theme=light'));
  assert.equal(app.navigation.length, 0);
  assert.equal(messages(app).length, 0, 'Never send before an explicit click');
});

test('A host that omits the local frame origin gets the launcher', async () => {
  const app = await createUiHarness('app');
  await initializeApp(app, 'light', { ...capabilities, sandbox: { csp: { frameDomains: [] } } });
  await openPanel(app);
  assert.equal(app.navigation.length, 0);
  assert.equal(app.element('launcher').hidden, false);
});

test('Launcher sends one current-chat request and waits for its matching editor receipt', async () => {
  const { app, url } = await fixture();
  click(app); click(app);
  assert.equal(calls(app, 'canvas_prepare_browser').length, 1);
  assert.equal(calls(app, 'canvas_prepare_browser')[0].params.arguments.theme, 'dark');
  await prepare(app, url);
  const message = messages(app)[0];
  assert.equal(messages(app).length, 1);
  assert.equal(message.params.role, 'user');
  assert.equal(message.params.options, undefined, 'Only the default current chat is targeted');
  assert.ok(message.params.content[0].text.includes(`${url}#theme=dark&launch=${launchId}`));
  assert.match(message.params.content[0].text, /open_in_codex/);
  assert.match(message.params.content[0].text, /不要启动系统默认浏览器/);
  app.host({ jsonrpc: '2.0', id: message.id, result: {} }); await settle();
  assert.match(app.element('launch-status').textContent, /已交给 Codex，等待内置画布连接/);
  assert.equal(app.element('open-canvas').disabled, true);
  app.timers.values().next().value(); await settle();
  const poll = calls(app, 'canvas_launch_status')[0];
  assert.equal(poll.params.arguments.launchId, launchId);
  app.host({ jsonrpc: '2.0', id: poll.id, result: { structuredContent: { launchId, state: 'connected' } } }); await settle();
  assert.match(app.element('launch-status').textContent, /^画布已连接/);
  assert.equal(app.element('open-canvas').disabled, false);
  assert.equal(calls(app, 'canvas_launch_browser').length, 0);
  assert.equal(app.hostMessages.some(item => item.data.method === 'ui/open-link'), false);
});

test('Unsupported messaging copies an in-app instruction instead of launching Chrome', async () => {
  for (const caps of [{ serverTools: {}, openLinks: {} }, { message: { text: {} } }]) {
    const app = await createUiHarness('app', { protocol: 'codex-sandbox:' });
    await initializeApp(app, 'light', caps);
    let url;
    if (caps.serverTools) url = await openPanel(app);
    else {
      url = 'http://127.0.0.1:1234/canvas/' + 'a'.repeat(43);
      app.host({ jsonrpc: '2.0', method: 'ui/notifications/tool-result', params: { structuredContent: { panelUrl: url } } });
    }
    assert.equal(app.element('open-canvas').textContent, '复制打开指令');
    click(app); await settle();
    assert.ok(app.clipboard[0].includes(url + '#theme=light'));
    assert.match(app.clipboard[0], /Codex 内置浏览器/);
    assert.match(app.element('launch-status').textContent, /粘贴到当前 Codex 聊天发送/);
    assert.equal(calls(app, 'canvas_prepare_browser').length, 0);
    assert.equal(messages(app).length, 0);
    assert.equal(app.navigation.length, 0);
  }
});

test('Copy fallback uses the current theme instruction and survives clipboard denial', async () => {
  const { app, url } = await fixture();
  app.element('copy-canvas').emit('click'); await settle();
  assert.ok(app.clipboard[0].includes(url + '#theme=dark'));
  let selected = false;
  app.navigator.clipboard.writeText = async () => { throw new Error('denied'); };
  app.element('canvas-url').select = () => { selected = true; };
  app.element('copy-canvas').emit('click'); await settle();
  assert.equal(selected, true);
  assert.match(app.element('launch-status').textContent, /手动复制/);
});

test('An uncertain message is never sent twice for the same opening receipt', async () => {
  const { app, url } = await fixture();
  click(app); await prepare(app, url);
  app.timers.values().next().value(); await settle();
  assert.match(app.element('launch-status').textContent, /未确认/);
  assert.equal(app.element('copy-fallback').open, true);
  click(app); await prepare(app, url);
  assert.equal(messages(app).length, 1, 'A timed-out message may already have reached the chat');
  assert.match(app.element('launch-status').textContent, /等待内置画布连接/);
});

test('Rejection and expired or mismatched receipts never claim an in-app connection', async () => {
  for (const state of ['rejected', 'unconfirmed', 'foreign']) {
    const { app, url } = await fixture();
    click(app); await prepare(app, url);
    const message = messages(app)[0];
    app.host({ jsonrpc: '2.0', id: message.id, ...(state === 'rejected' ? { error: { message: 'unsupported' } } : { result: {} }) });
    await settle();
    if (state !== 'rejected') {
      app.timers.values().next().value(); await settle();
      const poll = calls(app, 'canvas_launch_status')[0];
      app.host({ jsonrpc: '2.0', id: poll.id, result: { structuredContent: {
        launchId: state === 'foreign' ? 'c'.repeat(22) : launchId, state: state === 'foreign' ? 'connected' : state,
      } } }); await settle();
    }
    assert.match(app.element('launch-status').textContent, /未确认/);
    assert.equal(app.element('copy-fallback').open, true);
    assert.equal(messages(app).length, 1);
    assert.equal(app.element('open-canvas').disabled, false);
  }
});

test('Invalid or foreign prepared URLs cannot be handed to the host', async () => {
  for (const panelUrl of [undefined, 'https://foreign.example/canvas/' + 'a'.repeat(43),
    'http://127.0.0.1:1234/canvas/' + 'c'.repeat(43) + `#theme=dark&launch=${launchId}`]) {
    const { app } = await fixture(); click(app);
    app.host({ jsonrpc: '2.0', id: calls(app, 'canvas_prepare_browser')[0].id, result: { structuredContent: { launchId, state: 'pending', panelUrl } } });
    await settle();
    assert.equal(messages(app).length, 0);
    assert.match(app.element('launch-status').textContent, /未确认/);
  }
});

test('Launcher rejects foreign URLs before exposing an opening instruction', async () => {
  const app = await createUiHarness('app', { protocol: 'codex-sandbox:' });
  await initializeApp(app, 'light');
  const request = app.hostMessages.find(item => item.data.method === 'tools/call').data;
  app.host({ jsonrpc: '2.0', id: request.id, result: { structuredContent: { panelUrl: 'https://foreign.example/canvas/' + 'a'.repeat(43) } } });
  await settle();
  assert.equal(app.element('launcher').hidden, true);
  assert.equal(app.element('canvas-url').value, '');
  assert.equal(app.navigation.length, 0);
});
