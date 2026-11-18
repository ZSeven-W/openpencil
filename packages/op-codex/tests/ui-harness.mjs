import vm from 'node:vm';
import { readFile } from 'node:fs/promises';

export const settle = () => new Promise(resolve => setImmediate(resolve));

function events(target = {}) {
  const handlers = new Map();
  target.addEventListener = (type, handler) => {
    if (!handlers.has(type)) handlers.set(type, []);
    handlers.get(type).push(handler);
  };
  target.emit = (type, event = {}) => handlers.get(type)?.forEach(handler => handler(event));
  return target;
}

// Execute the shipped inline scripts with deterministic DOM/event boundaries.
// No browser, network, native editor, or user's document is involved.
export async function createUiHarness(kind, { dark = false, embedded = true, hash = '', openaiTheme, protocol = 'http:' } = {}) {
  const hostMessages = [], editorMessages = [], frameMessages = [], navigation = [], rpcCalls = [], clipboard = [];
  let rpcHandler = () => ({}), rawRpc = false;
  const elements = new Map();
  const element = id => {
    if (!elements.has(id)) elements.set(id, events({
      hidden: true, disabled: true, textContent: '', value: '', style: {}, dataset: {},
      focus() {}, select() {}, showModal() { this.open = true; }, close() { this.open = false; },
    }));
    return elements.get(id);
  };
  const frame = element(kind === 'app' ? 'panel' : 'editor');
  frame.contentWindow = { postMessage: (data, origin) => {
    frameMessages.push({ data, origin });
    if (kind === 'panel') editorMessages.push({ ...JSON.parse(data), targetOrigin: origin });
  } };
  let frameSrc;
  Object.defineProperty(frame, 'src', { get: () => frameSrc, set: value => { frameSrc = value; navigation.push(value); } });
  const media = events({ matches: dark });
  const root = { dataset: {}, style: { setProperty(key, value) { this[key] = value; } },
    setAttribute(key, value) { this[key] = value; } };
  const window = events({ location: { hash, protocol, origin: 'http://127.0.0.1:4321', href: 'http://127.0.0.1:4321/canvas/test' + hash },
    matchMedia: () => media });
  const parent = embedded ? { postMessage: (data, origin) => hostMessages.push({ data, origin }) } : window;
  window.parent = parent;
  window.postMessage = (data, origin) => hostMessages.push({ data, origin });
  if (openaiTheme !== undefined) window.openai = { theme: openaiTheme };
  const document = { documentElement: root, getElementById: element, hidden: false, referrer: '' };
  const timers = new Map(); let timerId = 0;
  const navigator = { clipboard: { writeText: async text => { clipboard.push(text); } } };
  const context = vm.createContext({ URL, URLSearchParams, window, document, navigator, location: window.location, console,
    grant: { editorOrigin: 'http://127.0.0.1:5678', token: 'test-token', rpcPath: '/canvas/test/rpc', source: '/test.op' },
    crypto: { randomUUID: () => 'test-snapshot' }, AbortSignal,
    setTimeout: fn => { timers.set(++timerId, fn); return timerId; }, clearTimeout: id => timers.delete(id),
    setInterval: () => ++timerId, clearInterval() {},
    fetch: async (url, options) => {
      rpcCalls.push({ url, options });
      const value = await rpcHandler(JSON.parse(options.body));
      return { ok: !rawRpc || !value.error, json: async () => rawRpc ? value : { structuredContent: value } };
    },
  });
  let script = await readFile(new URL(`../plugins/openpencil/ui/${kind === 'app' ? 'app.html' : 'panel.js'}`, import.meta.url), 'utf8');
  if (kind === 'app') script = script.match(/<script>([\s\S]*?)<\/script>/)[1];
  else {
    script += '\n' + await readFile(new URL('../plugins/openpencil/ui/selection.js', import.meta.url), 'utf8');
    script += '\n' + await readFile(new URL('../plugins/openpencil/ui/codegen.js', import.meta.url), 'utf8');
  }
  vm.runInContext(script, context);
  return { context, root, window, parent, frame, media, element, hostMessages, frameMessages, editorMessages, navigation, rpcCalls, clipboard, navigator, timers,
    rpc(handler, { raw = false } = {}) { rpcHandler = handler; rawRpc = raw; },
    theme: () => root.dataset.theme,
    systemTheme(dark) { media.matches = dark; media.emit('change', { matches: dark }); },
    host(data, origin = 'https://host.example') { window.emit('message', { source: parent, origin, data }); },
    editor(data, origin = 'http://127.0.0.1:5678') { window.emit('message', { source: frame.contentWindow, origin, data: JSON.stringify(data) }); },
    state: expression => vm.runInContext(expression, context),
  };
}

export async function initializeApp(ui, theme, capabilities = { serverTools: {}, updateModelContext: {} }) {
  const initialize = ui.hostMessages.find(item => item.data.method === 'ui/initialize').data;
  ui.host({ jsonrpc: '2.0', id: initialize.id, result: { hostCapabilities: capabilities, hostContext: theme ? { theme } : {} } });
  await settle();
}

export async function openPanel(ui) {
  const open = ui.hostMessages.find(item => item.data.method === 'tools/call').data;
  const url = 'http://127.0.0.1:1234/canvas/' + 'a'.repeat(43);
  ui.host({ jsonrpc: '2.0', id: open.id, result: { structuredContent: { panelUrl: url } } });
  await settle();
  return url;
}
