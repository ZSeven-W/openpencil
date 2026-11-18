const selectionError = (message, code) => Object.assign(new Error(message), { code });

function editorState(reply) {
  if (reply?.error || reply?.result?.isError) throw new Error('Cannot read the canvas selection.');
  const result = reply?.result;
  let state = result?.structuredContent;
  if (!state) {
    const text = result?.content?.find(item => item.type === 'text')?.text;
    try { state = JSON.parse(text); } catch { throw new Error('Invalid canvas selection response.'); }
  }
  const pageIndex = Number(state.active_page_index);
  if (!Number.isSafeInteger(pageIndex) || pageIndex < 0 || typeof state.active_page_id !== 'string' || typeof state.selection_ids !== 'string') {
    throw new Error('Invalid canvas selection state.');
  }
  return { pageId: state.active_page_id, pageIndex,
    nodeIds: [...new Set(state.selection_ids.split(',').filter(id => id && id !== '0'))].sort() };
}

export function sameSelection(a, b) {
  const right = Array.isArray(b?.nodeIds) ? [...b.nodeIds].sort() : [];
  return a?.pageId === b?.pageId && Array.isArray(a?.nodeIds) && Array.isArray(b?.nodeIds) &&
    a.nodeIds.length === b.nodeIds.length && [...a.nodeIds].sort().every((id, index) => id === right[index]);
}

export async function readCanvasSelection(session, { fresh = false } = {}) {
  const state = async () => editorState(await session.forward({ jsonrpc: '2.0', id: 'canvas-selection',
    method: 'tools/call', params: { name: 'get_editor_state', arguments: {} } }));
  const before = await state();
  const live = !fresh && session.selectionDocument ? await session.selectionDocument(before.pageIndex) : await session.document();
  const after = await state();
  if (!sameSelection(before, after) || before.pageIndex !== after.pageIndex || after.pageIndex !== live.activePageIndex) {
    throw selectionError('选区或页面正在切换，请确认选中的元素后重试。', 'SELECTION_TRANSITION');
  }
  const document = live.document;
  const page = document.pages?.length ? document.pages[after.pageIndex] : document;
  if (!page || (document.pages?.length && page.id !== after.pageId)) throw selectionError('页面尚在同步，请稍后重试。', 'SELECTION_TRANSITION');
  const nodes = new Map();
  const visit = (children, parentId = null) => {
    for (const node of children || []) {
      nodes.set(node.id, { id: node.id, name: node.name || node.type, type: node.type, parentId });
      visit(node.children, node.id);
    }
  };
  visit(page.children);
  if (after.nodeIds.some(id => !nodes.has(id))) throw selectionError('选中的元素已不存在，请重新选择。', 'SELECTION_STALE');
  return { live, selection: { ...after, count: after.nodeIds.length, nodes: after.nodeIds.map(id => nodes.get(id)) } };
}
