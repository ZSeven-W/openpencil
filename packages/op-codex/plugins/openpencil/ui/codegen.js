let codegenActive, codegenPolling = false, codegenRequest, codegenRevision;
const codegenSeen = new Set();
const codegenHandoffs = new Map();
function codegenReply(uiRequestId, fields) { send({ type: 'op-bridge/codegen-response', requestId: uiRequestId, ...fields }); }

async function generateCode(value) {
  if (!ready || codegenSeen.has(value.requestId)) return;
  if (typeof value.requestId !== 'string' || !/^host-\d+-\d+$/.test(value.requestId) || typeof value.framework !== 'string' || !Array.isArray(value.nodeIds)) return;
  codegenActive = value.requestId; codegenSeen.add(value.requestId);
  if (codegenSeen.size > 64) codegenSeen.delete(codegenSeen.values().next().value);
  try {
    const snapshot = await flush();
    if (codegenActive !== value.requestId) return;
    const job = await rpc('canvas_codegen_prepare', { uiRequestId: value.requestId, framework: value.framework,
      pageId: value.pageId, nodeIds: value.nodeIds, docJson: snapshot.docJson });
    if (codegenActive !== value.requestId) { await rpc('canvas_codegen_cancel', { uiRequestId: value.requestId }); return; }
    codegenRequest = job; codegenRevision = job.revision;
    codegenReply(value.requestId, { code: job.code, notice: '在 Codex 说“继续转代码”' });
    if (window.parent !== window) {
      const timer = setTimeout(() => {
        codegenHandoffs.delete(job.requestId);
        if (codegenRequest?.requestId !== job.requestId || codegenRequest.status === 'complete') return;
        codegenReply(job.uiRequestId, { notice: '投递未确认 · 请检查 Codex 对话' });
      }, 22_000);
      codegenHandoffs.set(job.requestId, { timer, job });
      window.parent.postMessage({ type: 'codex-canvas/codegen', requestId: job.requestId, prompt: job.prompt }, parentOrigin && parentOrigin !== 'null' ? parentOrigin : '*');
    }
  } catch (error) { if (codegenActive === value.requestId) codegenReply(value.requestId, { error: error.message }); }
}

window.addEventListener('message', event => {
  if (event.source === frame.contentWindow && event.origin === grant.editorOrigin && typeof event.data === 'string') {
    let value; try { value = JSON.parse(event.data); } catch { return; }
    if (value.type === 'op-bridge/codegen-cancel' && codegenActive === value.requestId) {
      codegenActive = undefined;
      if (codegenRequest?.uiRequestId === value.requestId) codegenRequest = undefined;
      void rpc('canvas_codegen_cancel', { uiRequestId: value.requestId }).catch(() => {});
    }
    if (value.type === 'op-bridge/codegen-request') void generateCode(value);
    return;
  }
  if (window.parent === window || event.source !== window.parent || event.data?.type !== 'codex-canvas/codegen-result') return;
  if (parentOrigin !== undefined && event.origin !== parentOrigin) return;
  const pending = codegenHandoffs.get(event.data.requestId);
  if (!pending) return;
  clearTimeout(pending.timer); codegenHandoffs.delete(event.data.requestId);
  if (codegenRequest?.requestId !== pending.job.requestId || codegenRequest.status === 'complete') return;
  const notice = event.data.status === 'accepted' ? '已交给 Codex 继续实现'
    : event.data.status === 'unsupported' ? '在 Codex 说“继续转代码”'
    : '投递未确认 · 请检查 Codex 对话';
  codegenReply(pending.job.uiRequestId, { notice });
});

async function refreshCodegen() {
  if (!ready || !codegenRequest || codegenPolling || document.hidden) return;
  codegenPolling = true;
  try {
    const job = await rpc('canvas_codegen_poll', { afterRevision: codegenRevision });
    if (!job || job.requestId !== codegenRequest.requestId) return;
    codegenRevision = job.revision; codegenRequest = job;
    codegenReply(job.uiRequestId, { code: job.code, notice: job.status === 'complete' ? 'Codex 已回填 · 项目见对话' : '基础代码预览' });
  } catch { /* Preserve the last usable source; reconnect does not replay a chat message. */ }
  finally { codegenPolling = false; }
}
setInterval(refreshCodegen, 1000);
