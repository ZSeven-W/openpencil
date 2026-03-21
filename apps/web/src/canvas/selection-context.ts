import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore, getActivePageChildren } from '@/stores/document-store'
import type { PenNode } from '@/types/pen'

/**
 * Pure utility module for depth-aware selection.
 * Determines which nodes are selectable at the current entered-frame depth.
 */

/** Returns the set of node IDs that are selectable at the current depth. */
export function getSelectableNodeIds(): Set<string> {
  const { enteredFrameId } = useCanvasStore.getState().selection
  const doc = useDocumentStore.getState().document

  if (!enteredFrameId) {
    // Root level: only top-level children of the active page are selectable
    const activePageId = useCanvasStore.getState().activePageId
    const children = getActivePageChildren(doc, activePageId)
    return new Set(children.map((n) => n.id))
  }

  const frame = useDocumentStore.getState().getNodeById(enteredFrameId)
  if (!frame || !('children' in frame) || !frame.children) {
    return new Set()
  }
  return new Set(frame.children.map((n) => n.id))
}

/**
 * Given a Fabric target's penNodeId, resolve it to the selectable node
 * at the current depth. Walks up the parent chain until it finds a node
 * in the selectable set.
 *
 * Returns null if the target is entirely outside the current context
 * (e.g. belongs to a different root frame when inside an entered frame).
 */
export function resolveTargetAtDepth(nodeId: string): string | null {
  const selectableIds = getSelectableNodeIds()

  // Direct match
  if (selectableIds.has(nodeId)) return nodeId

  // Handle virtual instance child IDs (refId__childId)
  if (nodeId.includes('__')) {
    const refId = nodeId.substring(0, nodeId.indexOf('__'))
    if (selectableIds.has(refId)) return refId
    // Walk up from the RefNode
    let cur: string | undefined = refId
    while (cur) {
      const parent = useDocumentStore.getState().getParentOf(cur)
      if (!parent) break
      if (selectableIds.has(parent.id)) return parent.id
      cur = parent.id
    }
  }

  // Walk up parent chain
  let currentId: string | undefined = nodeId
  while (currentId) {
    const parent = useDocumentStore.getState().getParentOf(currentId)
    if (!parent) break
    if (selectableIds.has(parent.id)) return parent.id
    currentId = parent.id
  }

  return null
}

/** Check whether a node is a container that can be "entered" via double-click. */
export function isEnterableContainer(nodeId: string): boolean {
  const node = useDocumentStore.getState().getNodeById(nodeId)
  if (!node) return false
  if (node.type !== 'frame' && node.type !== 'group') return false
  if (!('children' in node) || !node.children || node.children.length === 0) return false
  return true
}

/** Return the direct children IDs of a container node. */
export function getChildIds(nodeId: string): Set<string> {
  const node = useDocumentStore.getState().getNodeById(nodeId)
  if (!node || !('children' in node) || !node.children) return new Set()
  return new Set(node.children.map((n: PenNode) => n.id))
}
