import { useDocumentStore } from '@/stores/document-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { getActivePageChildren, setActivePageChildren } from '@/stores/document-tree-utils'

/**
 * Force the canvas sync subscriber to re-run by creating a new page children
 * reference. The old pattern `{ ...doc, children: [...doc.children] }` only
 * touched root-level children which are empty under the pages architecture.
 */
export function forcePageResync() {
  const doc = useDocumentStore.getState().document
  const activePageId = useCanvasStore.getState().activePageId
  const children = getActivePageChildren(doc, activePageId)
  useDocumentStore.setState({
    document: setActivePageChildren(doc, activePageId, [...children]),
  })
}
