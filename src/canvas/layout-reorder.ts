import type * as fabric from 'fabric'
import { useDocumentStore } from '@/stores/document-store'
import { forcePageResync } from './canvas-sync-utils'
import type { PenNode, ContainerProps } from '@/types/pen'
import type { FabricObjectWithPenId } from './canvas-object-factory'
import { setFabricSyncLock } from './canvas-sync-lock'
import { nodeRenderInfo } from './use-canvas-sync'
import { setInsertionIndicator } from './insertion-indicator'

// ---------------------------------------------------------------------------
// Session state
// ---------------------------------------------------------------------------

interface LayoutDragSession {
  nodeId: string
  parentId: string
  parentLayout: 'vertical' | 'horizontal'
  siblingIds: string[]
  originalIndex: number
}

let activeSession: LayoutDragSession | null = null

// ---------------------------------------------------------------------------
// Padding helper (duplicated from use-canvas-sync to avoid circular deps)
// ---------------------------------------------------------------------------

interface Padding {
  top: number
  right: number
  bottom: number
  left: number
}

function resolvePadding(
  padding:
    | number
    | [number, number]
    | [number, number, number, number]
    | string
    | undefined,
): Padding {
  if (!padding || typeof padding === 'string')
    return { top: 0, right: 0, bottom: 0, left: 0 }
  if (typeof padding === 'number')
    return { top: padding, right: padding, bottom: padding, left: padding }
  if (padding.length === 2)
    return {
      top: padding[0],
      right: padding[1],
      bottom: padding[0],
      left: padding[1],
    }
  return {
    top: padding[0],
    right: padding[1],
    bottom: padding[2],
    left: padding[3],
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function buildFabObjectMap(
  canvas: fabric.Canvas,
): Map<string, FabricObjectWithPenId> {
  const map = new Map<string, FabricObjectWithPenId>()
  for (const obj of canvas.getObjects() as FabricObjectWithPenId[]) {
    if (obj.penNodeId) map.set(obj.penNodeId, obj)
  }
  return map
}

/**
 * Calculate insertion index based on the dragged object's main-axis center
 * relative to each sibling's midpoint.  The returned index is in "after
 * removal" space (siblingIds already excludes the dragged node) and maps
 * directly to the `index` parameter of `moveNode`.
 */
function calcInsertionIndex(
  obj: FabricObjectWithPenId,
  fabObjectMap: Map<string, FabricObjectWithPenId>,
): number {
  if (!activeSession) return 0

  const { siblingIds, parentLayout } = activeSession
  const isVertical = parentLayout === 'vertical'

  const objMainCenter = isVertical
    ? (obj.top ?? 0) + ((obj.height ?? 0) * (obj.scaleY ?? 1)) / 2
    : (obj.left ?? 0) + ((obj.width ?? 0) * (obj.scaleX ?? 1)) / 2

  let insertIndex = siblingIds.length

  for (let i = 0; i < siblingIds.length; i++) {
    const sibObj = fabObjectMap.get(siblingIds[i])
    if (!sibObj) continue
    const sibMid = isVertical
      ? (sibObj.top ?? 0) + ((sibObj.height ?? 0) * (sibObj.scaleY ?? 1)) / 2
      : (sibObj.left ?? 0) +
        ((sibObj.width ?? 0) * (sibObj.scaleX ?? 1)) / 2
    if (objMainCenter < sibMid) {
      insertIndex = i
      break
    }
  }

  return insertIndex
}

// ---------------------------------------------------------------------------
// Session lifecycle
// ---------------------------------------------------------------------------

/**
 * Try to begin a layout reorder drag.
 * Returns true if the node is a layout child and the session was started.
 */
export function beginLayoutDrag(nodeId: string): boolean {
  const info = nodeRenderInfo.get(nodeId)
  if (!info?.isLayoutChild) return false

  const { getParentOf } = useDocumentStore.getState()
  const parent = getParentOf(nodeId)
  if (!parent) return false

  const container = parent as PenNode & ContainerProps
  const layout = container.layout
  if (!layout || layout === 'none') return false

  const children = 'children' in parent ? parent.children ?? [] : []
  const originalIndex = children.findIndex((c) => c.id === nodeId)
  if (originalIndex === -1) return false

  activeSession = {
    nodeId,
    parentId: parent.id,
    parentLayout: layout as 'vertical' | 'horizontal',
    siblingIds: children.filter((c) => c.id !== nodeId).map((c) => c.id),
    originalIndex,
  }

  return true
}

/**
 * Update insertion indicator position during drag.
 */
export function updateLayoutDrag(
  obj: FabricObjectWithPenId,
  canvas: fabric.Canvas,
): void {
  if (!activeSession) return

  const { siblingIds, parentId, parentLayout } = activeSession
  const isVertical = parentLayout === 'vertical'
  const fabObjectMap = buildFabObjectMap(canvas)
  const insertIndex = calcInsertionIndex(obj, fabObjectMap)

  const { getNodeById } = useDocumentStore.getState()
  const parent = getNodeById(parentId) as
    | (PenNode & ContainerProps)
    | undefined
  if (!parent) return

  const pad = resolvePadding(parent.padding)
  const gap = typeof parent.gap === 'number' ? parent.gap : 0
  const parentInfo = nodeRenderInfo.get(parentId)
  const parentAbsX = (parentInfo?.parentOffsetX ?? 0) + (parent.x ?? 0)
  const parentAbsY = (parentInfo?.parentOffsetY ?? 0) + (parent.y ?? 0)
  const parentW = typeof parent.width === 'number' ? parent.width : 0
  const parentH = typeof parent.height === 'number' ? parent.height : 0

  if (isVertical) {
    let indicatorY: number
    if (siblingIds.length === 0) {
      indicatorY = parentAbsY + pad.top
    } else if (insertIndex === 0) {
      const firstSib = fabObjectMap.get(siblingIds[0])
      indicatorY = firstSib
        ? (firstSib.top ?? 0) - gap / 2
        : parentAbsY + pad.top
    } else if (insertIndex >= siblingIds.length) {
      const lastSib = fabObjectMap.get(siblingIds[siblingIds.length - 1])
      indicatorY = lastSib
        ? (lastSib.top ?? 0) +
          (lastSib.height ?? 0) * (lastSib.scaleY ?? 1) +
          gap / 2
        : parentAbsY + parentH - pad.bottom
    } else {
      const prev = fabObjectMap.get(siblingIds[insertIndex - 1])
      const next = fabObjectMap.get(siblingIds[insertIndex])
      const prevBottom = prev
        ? (prev.top ?? 0) + (prev.height ?? 0) * (prev.scaleY ?? 1)
        : 0
      const nextTop = next ? (next.top ?? 0) : 0
      indicatorY = (prevBottom + nextTop) / 2
    }

    setInsertionIndicator({
      x: parentAbsX + pad.left,
      y: indicatorY,
      length: parentW - pad.left - pad.right,
      orientation: 'horizontal',
    })
  } else {
    let indicatorX: number
    if (siblingIds.length === 0) {
      indicatorX = parentAbsX + pad.left
    } else if (insertIndex === 0) {
      const firstSib = fabObjectMap.get(siblingIds[0])
      indicatorX = firstSib
        ? (firstSib.left ?? 0) - gap / 2
        : parentAbsX + pad.left
    } else if (insertIndex >= siblingIds.length) {
      const lastSib = fabObjectMap.get(siblingIds[siblingIds.length - 1])
      indicatorX = lastSib
        ? (lastSib.left ?? 0) +
          (lastSib.width ?? 0) * (lastSib.scaleX ?? 1) +
          gap / 2
        : parentAbsX + parentW - pad.right
    } else {
      const prev = fabObjectMap.get(siblingIds[insertIndex - 1])
      const next = fabObjectMap.get(siblingIds[insertIndex])
      const prevRight = prev
        ? (prev.left ?? 0) + (prev.width ?? 0) * (prev.scaleX ?? 1)
        : 0
      const nextLeft = next ? (next.left ?? 0) : 0
      indicatorX = (prevRight + nextLeft) / 2
    }

    setInsertionIndicator({
      x: indicatorX,
      y: parentAbsY + pad.top,
      length: parentH - pad.top - pad.bottom,
      orientation: 'vertical',
    })
  }

  canvas.requestRenderAll()
}

/**
 * End the layout drag: clear manual x/y, reorder the node, force re-sync.
 */
export function endLayoutDrag(
  obj: FabricObjectWithPenId,
  canvas: fabric.Canvas,
): void {
  if (!activeSession) return

  const { nodeId, parentId, originalIndex } = activeSession
  const fabObjectMap = buildFabObjectMap(canvas)
  const newIndex = calcInsertionIndex(obj, fabObjectMap)

  setFabricSyncLock(true)

  // Clear manual position so layout engine takes over
  useDocumentStore.getState().updateNode(nodeId, {
    x: undefined,
    y: undefined,
  } as Partial<PenNode>)

  // Reorder if the position actually changed
  if (newIndex !== originalIndex) {
    useDocumentStore.getState().moveNode(nodeId, parentId, newIndex)
  }

  setFabricSyncLock(false)

  // Force re-sync: create new children reference so the subscription fires
  forcePageResync()

  activeSession = null
  setInsertionIndicator(null)
  canvas.requestRenderAll()
}

/** Cancel the layout drag session (safety cleanup). */
export function cancelLayoutDrag(): void {
  activeSession = null
  setInsertionIndicator(null)
}

/** Check if a layout drag session is currently active. */
export function isLayoutDragActive(): boolean {
  return activeSession !== null
}
