import { useDocumentStore } from '@/stores/document-store';
import { useCanvasStore } from '@/stores/canvas-store';
import { getActivePageChildren } from '@/stores/document-tree-utils';

/**
 * Subscribe to the active page's children array reference.
 * Calls `onSync` only when the children reference changes (not on
 * unrelated store mutations like fileName or isDirty).
 *
 * Returns an unsubscribe function.
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
