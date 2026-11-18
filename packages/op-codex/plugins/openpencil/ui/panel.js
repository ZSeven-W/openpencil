const el = id => document.getElementById(id);
const frame = el('editor');
let ready = false, saving = false, dirty = false, source = grant.source;
let generation = 0, revision = 0, savedVersion = null, checkpointTimer;
let refreshingCanvas = false;
let connectionInterrupted = false;
const errors = new Map();
const snapshots = new Map();
const systemTheme = window.matchMedia?.('(prefers-color-scheme: dark)');
const validTheme = value => value === 'light' || value === 'dark';
const seedTheme = new URLSearchParams(window.location.hash.slice(1)).get('theme');
const browserLaunchId = new URLSearchParams(window.location.hash.slice(1)).get('launch');
let launchAcknowledged = false;
let parentTheme = validTheme(seedTheme) ? seedTheme : undefined;
let compatibilityTheme = validTheme(window.openai?.theme) ? window.openai.theme : undefined;
let colorScheme, parentOrigin, editorInitialized = false;

function renderError() {
  // Background connection failures must not hide a failed save or sync conflict.
  const visible = [...errors.entries()].filter(([, error]) => !error.dismissed);
  const current = (visible.findLast(([kind]) => kind !== 'connection') || visible.at(-1))?.[1];
  el('error-message').textContent = current?.message || '';
  el('error').hidden = !current;
}
function fail(error, kind = 'operation') {
  const message = error.message || String(error), previous = errors.get(kind);
  errors.set(kind, { message, dismissed: previous?.message === message && previous.dismissed });
  renderError();
}
function clearError(kind) { errors.delete(kind); renderError(); }
function status(text) { el('status').textContent = text; }
function restoreStatus() {
  status(errors.has('sync') ? '同步冲突' : errors.has('save') ? '保存未完成' : connectionInterrupted ? '画布连接中断' : dirty ? '未保存' : '画布已连接');
}
function send(value) { frame.contentWindow?.postMessage(JSON.stringify(value), grant.editorOrigin); }
function sendTheme() { if (editorInitialized) send({ type:'op-bridge/theme', colorScheme }); }
function applyTheme() {
  const nextTheme = parentTheme || compatibilityTheme || (systemTheme?.matches ? 'dark' : 'light');
  if (colorScheme === nextTheme) return;
  colorScheme = nextTheme;
  document.documentElement.dataset.theme = colorScheme;
  document.documentElement.style.colorScheme = colorScheme;
  sendTheme();
}
function notifyParentReady() {
  if (window.parent !== window) window.parent.postMessage({ type:'codex-canvas/ready' }, parentOrigin && parentOrigin !== 'null' ? parentOrigin : '*');
}
function initialize() {
  send({ type: 'op-bridge/init', token: grant.token });
  editorInitialized = true;
  sendTheme();
}
systemTheme?.addEventListener('change', applyTheme);
window.addEventListener('openai:set_globals', event => {
  const theme = event.detail?.globals?.theme;
  if (!validTheme(theme)) return;
  compatibilityTheme = theme;
  applyTheme();
});
applyTheme();
async function rpc(name, args = {}) {
  const response = await fetch(grant.rpcPath, { method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name, arguments: args }), signal: AbortSignal.timeout(15_000) });
  const value = await response.json();
  if (!response.ok || value.error || value.isError) throw Object.assign(new Error(value.error || value.content?.[0]?.text || '画布操作失败'), { code: value.code || value.structuredContent?.code });
  return value.structuredContent;
}
function updateSource() { el('document-label').textContent = source || '未命名设计 · 独立文档'; el('document-label').title = source || ''; }
function flush() {
  const requestId = crypto.randomUUID();
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => { snapshots.delete(requestId); reject(new Error('画布同步超时。请检查内容后重新保存，操作不会自动重试。')); }, 15_000);
    snapshots.set(requestId, { resolve, reject, timer });
    send({ type: 'op-bridge/snapshot', purpose: 'save', requestId });
  });
}
async function save(path = source) {
  if (!ready || saving) return;
  if (!path) { el('save-dialog').showModal(); el('file-path').focus(); return; }
  saving = true; el('save').disabled = el('save-as').disabled = true; status('正在保存…'); clearError('save');
  try {
    const snapshot = await flush();
    const result = await rpc('canvas_save', { filePath: path, docJson: snapshot.docJson });
    source = result.filePath; savedVersion = result.version;
    // A later edit must stay dirty even when an earlier Save completes.
    if (generation === snapshot.generation && revision === snapshot.revision) dirty = false;
    send({ type: 'op-bridge/save-committed', generation: snapshot.generation, revision: snapshot.revision });
    clearError('sync'); updateSource(); status(dirty ? '有新的未保存修改' : '已保存');
  } catch (error) { fail(error, 'save'); status('保存未完成'); }
  finally { saving = false; el('save').disabled = el('save-as').disabled = !ready; }
}
function checkpointSoon() {
  if (checkpointTimer) return;
  checkpointTimer = setTimeout(() => { checkpointTimer = undefined; if (!saving && ready) rpc('canvas_checkpoint').then(() => clearError('checkpoint')).catch(error => fail(error, 'checkpoint')); }, 1000);
}
window.addEventListener('message', event => {
  if (window.parent !== window && event.source === window.parent && event.data?.type === 'codex-canvas/theme') {
    if (parentOrigin !== undefined && event.origin !== parentOrigin) return;
    if (validTheme(event.data.colorScheme)) {
      parentOrigin = event.origin;
      parentTheme = event.data.colorScheme;
      applyTheme();
    }
    return;
  }
  if (event.source !== frame.contentWindow || event.origin !== grant.editorOrigin || typeof event.data !== 'string') return;
  let value; try { value = JSON.parse(event.data); } catch { return; }
  if (value.type === 'op-bridge/listening') initialize();
  if (value.type === 'op-bridge/ready') {
    if (!Number.isSafeInteger(value.generation) || !Number.isSafeInteger(value.revision)) return;
    ready = true; generation = value.generation; revision = value.revision;
    el('loading').hidden = true; el('save').disabled = el('save-as').disabled = false;
    connectionInterrupted = false; clearError('connection'); restoreStatus();
    sendTheme(); notifyParentReady();
    if (!launchAcknowledged && /^[A-Za-z0-9_-]{22}$/.test(browserLaunchId || '')) {
      launchAcknowledged = true;
      void rpc('canvas_launch_ready', { launchId: browserLaunchId }).catch(() => {});
    }
  }
  if (value.type === 'op-bridge/dirty-changed') {
    if (!Number.isSafeInteger(value.generation) || !Number.isSafeInteger(value.revision)) return;
    generation = value.generation; revision = value.revision; dirty = Boolean(value.dirty) || dirty;
    if (!saving && dirty) restoreStatus(); checkpointSoon();
  }
  if (value.type === 'op-bridge/sync-conflict') { dirty = true; status('同步冲突'); fail(new Error('设计在同步过程中发生变化。请在编辑器中处理冲突，再保存。'), 'sync'); }
  if (value.type === 'op-bridge/snapshot-result' || value.type === 'op-bridge/snapshot-conflict') {
    const pending = snapshots.get(value.requestId);
    if (!pending) return;
    clearTimeout(pending.timer); snapshots.delete(value.requestId);
    if (value.type === 'op-bridge/snapshot-conflict') pending.reject(new Error('设计已发生变化，保存已停止。请同步后重试。'));
    else pending.resolve(value);
  }
  if (value.type === 'op-shell/save') void save();
});
frame.addEventListener('load', initialize);
frame.src = `${grant.editorOrigin}/?embed=vscode&chat=host&locale=zh-CN&theme=${colorScheme}`;
notifyParentReady();
updateSource();
const startup = setInterval(() => { if (ready) clearInterval(startup); else initialize(); }, 1000);
setTimeout(() => { if (!ready) { clearInterval(startup); connectionInterrupted = true; fail(new Error('编辑器尚未连接。请检查运行包、页面加载或插件连接后重新打开画布。'), 'connection'); } }, 30_000);
el('error-dismiss').onclick = () => { for (const error of errors.values()) error.dismissed = true; renderError(); };
el('save').onclick = () => void save();
el('save-as').onclick = () => { el('file-path').value = source || ''; el('save-dialog').showModal(); };
el('cancel').onclick = () => el('save-dialog').close();
el('save-form').onsubmit = event => { event.preventDefault(); const path = el('file-path').value.trim(); if (path) { el('save-dialog').close(); void save(path); } };
window.addEventListener('keydown', event => { if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') { event.preventDefault(); void save(); } });
window.addEventListener('beforeunload', event => { if (dirty) { event.preventDefault(); event.returnValue = ''; } });
async function refreshCanvas() {
  if (!ready || saving || refreshingCanvas || document.hidden) return;
  refreshingCanvas = true;
  try {
    const result = await rpc('canvas_status');
    source = result.filePath || source; updateSource();
    renderSelection(result.selection);
    clearError('selection'); clearError('connection');
    if (connectionInterrupted) { connectionInterrupted = false; restoreStatus(); }
    if (savedVersion === null) savedVersion = result.version;
    else if (result.version !== savedVersion) { dirty = true; restoreStatus(); checkpointSoon(); }
    if (window.parent !== window) window.parent.postMessage({ type: 'codex-canvas/context', filePath: source, selection: result.selection }, parentOrigin && parentOrigin !== 'null' ? parentOrigin : '*');
  } catch (error) {
    selectionReadFailed(error);
    if (!isSelectionReadError(error)) { connectionInterrupted = true; fail(error, 'connection'); restoreStatus(); }
    if (window.parent !== window) window.parent.postMessage({ type: 'codex-canvas/context', filePath: source, selection: null }, parentOrigin && parentOrigin !== 'null' ? parentOrigin : '*');
  }
  finally { refreshingCanvas = false; }
}
setInterval(refreshCanvas, 750);
