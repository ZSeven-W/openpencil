function isSelectionReadError(error) {
  // A running bridge may still use the earlier, untyped response until reconnect.
  return (typeof error.code === 'string' && error.code.startsWith('SELECTION_')) || error.message === '选区或页面正在切换，请确认选中的元素后重试。';
}

function selectionReadFailed(error) {
  renderSelection(null);
  el('selection-label').textContent = isSelectionReadError(error)
    ? error.code === 'SELECTION_STALE' ? '请重新选择元素' : '正在同步选区…'
    : '选区暂不可用';
}

function renderSelection(value) {
  const selection = value?.count > 0 && Array.isArray(value.nodeIds) && value.count === value.nodeIds.length ? value : null;
  const names = selection?.nodes?.map(node => node.name || node.id).join('、') || '';
  const text = selection ? `已选 ${selection.count} 个元素${names ? ` · ${names}` : ''}` : '未选中元素';
  el('selection-label').textContent = text;
  el('selection-label').title = text;
}
