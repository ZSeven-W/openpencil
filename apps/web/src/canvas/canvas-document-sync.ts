import { useDocumentStore } from '@/stores/document-store';
import { useCanvasStore } from '@/stores/canvas-store';
import { getActivePageChildren } from '@/stores/document-tree-utils';

/**
 * Subscribe
 * 到活动页面的子数组引用。 Calls `onSync` 仅当子级引用更改时（不适用于不相关的存储突变，如
 * fileName 或 isDirty）。 Returns 取消订阅功能。
 *
 *
 */
export function subscribeToActivePageChildren(onSync: () => void): () => void {
  let prevChildren = getActivePageChildren(
    useDocumentStore.getState().document,
    useCanvasStore.getState().activePageId,
  );
  return useDocumentStore.subscribe((state) => {
    const children = getActivePageChildren(state.document, useCanvasStore.getState().activePageId);
    if (children !== prevChildren) {
      prevChildren = children;
      onSync();
    }
  });
}
