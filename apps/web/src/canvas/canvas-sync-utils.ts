import { useDocumentStore } from '@/stores/document-store';
import { useCanvasStore } from '@/stores/canvas-store';
import { getActivePageChildren, setActivePageChildren } from '@/stores/document-tree-utils';

/**
 * Force 画布同步订阅
 * 者通过创建新的页面子引用来重新运行。 The 旧模式 `{ ...doc, children: [...doc.children] }`
 * 只触及根级子级，这些子级在页面架构下是空的。
 */
export function forcePageResync() {
  const doc = useDocumentStore.getState().document;
  const activePageId = useCanvasStore.getState().activePageId;
  const children = getActivePageChildren(doc, activePageId);
  useDocumentStore.setState({
    document: setActivePageChildren(doc, activePageId, [...children]),
  });
}
