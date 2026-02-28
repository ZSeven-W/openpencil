import type * as fabric from 'fabric'
import { useDocumentStore } from '@/stores/document-store'
import { forcePageResync } from './canvas-sync-utils'
import type { FabricObjectWithPenId } from './canvas-object-factory'
import { setFabricSyncLock } from './canvas-sync-lock'
import { layoutContainerBounds, rootFrameBounds } from './use-canvas-sync'
import type { LayoutContainerInfo } from './use-canvas-sync'
import { setInsertionIndicator, setContainerHighlight } from './insertion-indicator'

// ---------------------------------------------------------------------------
// Session state
// ---------------------------------------------------------------------------

interface DragIntoSession {
  nodeId: string
  nodeIds?: string[]
  targetContainerId: string
  insertionIndex: number
  /** Whether the target is a layout container (vertical/horizontal) vs a plain frame */
  isLayoutTarget: boolean
}

let activeSession: DragIntoSession | null = null

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
 * Compute the insertion index inside a layout container based on the
 * dragged object's main-axis center relative to each child's midpoint.
 */
function calcInsertionIndex(
  objMainCenter: number,
  containerInfo: LayoutContainerInfo,
  childIds: string[],
  fabObjectMap: Map<string, FabricObjectWithPenId>,
): number {
  const isVertical = containerInfo.layout === 'vertical'

  let insertIndex = childIds.length
  for (let i = 0; i < childIds.length; i++) {
    const sibObj = fabObjectMap.get(childIds[i])
    if (!sibObj) continue
    const sibMid = isVertical
      ? (sibObj.top ?? 0) + ((sibObj.height ?? 0) * (sibObj.scaleY ?? 1)) / 2
      : (sibObj.left ?? 0) + ((sibObj.width ?? 0) * (sibObj.scaleX ?? 1)) / 2
    if (objMainCenter < sibMid) {
      insertIndex = i
      break
    }
  }

  return insertIndex
}

/**
 * Compute the insertion indicator position for a given container and index.
 */
function computeIndicator(
  containerInfo: LayoutContainerInfo,
  childIds: string[],
  insertIndex: number,
  fabObjectMap: Map<string, FabricObjectWithPenId>,
) {
  const { x, y, w, h, layout, padding: pad, gap } = containerInfo
  const isVertical = layout === 'vertical'

  if (isVertical) {
    let indicatorY: number
    if (childIds.length === 0) {
      indicatorY = y + pad.top
    } else if (insertIndex === 0) {
      const firstSib = fabObjectMap.get(childIds[0])
      indicatorY = firstSib
        ? (firstSib.top ?? 0) - gap / 2
        : y + pad.top
    } else if (insertIndex >= childIds.length) {
      const lastSib = fabObjectMap.get(childIds[childIds.length - 1])
      indicatorY = lastSib
        ? (lastSib.top ?? 0) +
          (lastSib.height ?? 0) * (lastSib.scaleY ?? 1) +
          gap / 2
        : y + h - pad.bottom
    } else {
      const prev = fabObjectMap.get(childIds[insertIndex - 1])
      const next = fabObjectMap.get(childIds[insertIndex])
      const prevBottom = prev
        ? (prev.top ?? 0) + (prev.height ?? 0) * (prev.scaleY ?? 1)
        : 0
      const nextTop = next ? (next.top ?? 0) : 0
      indicatorY = (prevBottom + nextTop) / 2
    }

    setInsertionIndicator({
      x: x + pad.left,
      y: indicatorY,
      length: w - pad.left - pad.right,
      orientation: 'horizontal',
    })
  } else {
    let indicatorX: number
    if (childIds.length === 0) {
      indicatorX = x + pad.left
    } else if (insertIndex === 0) {
      const firstSib = fabObjectMap.get(childIds[0])
      indicatorX = firstSib
        ? (firstSib.left ?? 0) - gap / 2
        : x + pad.left
    } else if (insertIndex >= childIds.length) {
      const lastSib = fabObjectMap.get(childIds[childIds.length - 1])
      indicatorX = lastSib
        ? (lastSib.left ?? 0) +
          (lastSib.width ?? 0) * (lastSib.scaleX ?? 1) +
          gap / 2
        : x + w - pad.right
    } else {
      const prev = fabObjectMap.get(childIds[insertIndex - 1])
      const next = fabObjectMap.get(childIds[insertIndex])
      const prevRight = prev
        ? (prev.left ?? 0) + (prev.width ?? 0) * (prev.scaleX ?? 1)
        : 0
      const nextLeft = next ? (next.left ?? 0) : 0
      indicatorX = (prevRight + nextLeft) / 2
    }

    setInsertionIndicator({
      x: indicatorX,
      y: y + pad.top,
      length: h - pad.top - pad.bottom,
      orientation: 'vertical',
    })
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Called during object:moving — detect if the dragged object is over a
 * layout container and show insertion indicator + container highlight.
 */
export function checkDragIntoTarget(
  obj: FabricObjectWithPenId,
  canvas: fabric.Canvas,
): void {
  const nodeId = obj.penNodeId
  if (!nodeId) return

  const store = useDocumentStore.getState()

  // Object center point for hit detection
  const objCenterX =
    (obj.left ?? 0) + ((obj.width ?? 0) * (obj.scaleX ?? 1)) / 2
  const objCenterY =
    (obj.top ?? 0) + ((obj.height ?? 0) * (obj.scaleY ?? 1)) / 2

  // Find the best (innermost) container containing the center.
  // Check layout containers first (they support insertion indicators),
  // then non-layout root frames (just container highlight).
  const parent = store.getParentOf(nodeId)

  let bestContainerId: string | null = null
  let bestInfo: LayoutContainerInfo | null = null
  let bestFrameBounds: { x: number; y: number; w: number; h: number } | null = null
  let bestArea = Infinity
  let bestIsLayout = false

  // 1. Check layout containers (preferred — support insertion indicators)
  for (const [containerId, info] of layoutContainerBounds) {
    if (parent?.id === containerId) continue
    if (store.isDescendantOf(containerId, nodeId)) continue
    if (containerId === nodeId) continue

    if (
      objCenterX >= info.x &&
      objCenterX <= info.x + info.w &&
      objCenterY >= info.y &&
      objCenterY <= info.y + info.h
    ) {
      const area = info.w * info.h
      if (area < bestArea) {
        bestArea = area
        bestContainerId = containerId
        bestInfo = info
        bestIsLayout = true
      }
    }
  }

  // 2. Check non-layout root frames (just container highlight, no indicator)
  for (const [frameId, bounds] of rootFrameBounds) {
    if (layoutContainerBounds.has(frameId)) continue // already checked above
    if (parent?.id === frameId) continue
    if (store.isDescendantOf(frameId, nodeId)) continue
    if (frameId === nodeId) continue

    if (
      objCenterX >= bounds.x &&
      objCenterX <= bounds.x + bounds.w &&
      objCenterY >= bounds.y &&
      objCenterY <= bounds.y + bounds.h
    ) {
      const area = bounds.w * bounds.h
      if (area < bestArea) {
        bestArea = area
        bestContainerId = frameId
        bestInfo = null
        bestFrameBounds = bounds
        bestIsLayout = false
      }
    }
  }

  if (bestContainerId && bestIsLayout && bestInfo) {
    // Layout container: show insertion indicator + container highlight
    const fabObjectMap = buildFabObjectMap(canvas)
    const container = store.getNodeById(bestContainerId)
    const childIds =
      container && 'children' in container && container.children
        ? container.children.map((c) => c.id)
        : []

    const isVertical = bestInfo.layout === 'vertical'
    const mainCenter = isVertical ? objCenterY : objCenterX
    const insertIndex = calcInsertionIndex(
      mainCenter,
      bestInfo,
      childIds,
      fabObjectMap,
    )

    activeSession = {
      nodeId,
      targetContainerId: bestContainerId,
      insertionIndex: insertIndex,
      isLayoutTarget: true,
    }

    computeIndicator(bestInfo, childIds, insertIndex, fabObjectMap)
    setContainerHighlight({
      x: bestInfo.x,
      y: bestInfo.y,
      w: bestInfo.w,
      h: bestInfo.h,
    })

    canvas.requestRenderAll()
  } else if (bestContainerId && !bestIsLayout) {
    // Non-layout frame: show container highlight only, append at end
    const bounds = bestFrameBounds ?? rootFrameBounds.get(bestContainerId)!
    const container = store.getNodeById(bestContainerId)
    const childCount =
      container && 'children' in container && container.children
        ? container.children.length
        : 0

    activeSession = {
      nodeId,
      targetContainerId: bestContainerId,
      insertionIndex: childCount,
      isLayoutTarget: false,
    }

    setInsertionIndicator(null)
    setContainerHighlight({
      x: bounds.x,
      y: bounds.y,
      w: bounds.w,
      h: bounds.h,
    })

    canvas.requestRenderAll()
  } else {
    // Not over any container — clear
    if (activeSession) {
      activeSession = null
      setInsertionIndicator(null)
      setContainerHighlight(null)
      canvas.requestRenderAll()
    }
  }
}

/**
 * Commit the drag-into: reparent the node into the target container.
 * Returns true if a reparent was performed.
 */
export function commitDragInto(
  obj: FabricObjectWithPenId,
  canvas: fabric.Canvas,
): boolean {
  if (!activeSession) return false

  const { nodeId, targetContainerId, insertionIndex, isLayoutTarget } = activeSession

  setFabricSyncLock(true)

  if (isLayoutTarget) {
    // Clear manual position so layout engine takes over
    useDocumentStore.getState().updateNode(nodeId, {
      x: undefined,
      y: undefined,
    } as Partial<import('@/types/pen').PenNode>)
  } else {
    // Non-layout frame: convert absolute position to relative
    const targetBounds = rootFrameBounds.get(targetContainerId)
    if (targetBounds) {
      useDocumentStore.getState().updateNode(nodeId, {
        x: (obj.left ?? 0) - targetBounds.x,
        y: (obj.top ?? 0) - targetBounds.y,
      })
    }
  }

  // Reparent into the target container
  useDocumentStore.getState().moveNode(nodeId, targetContainerId, insertionIndex)

  setFabricSyncLock(false)

  // Force re-sync: must re-read state after mutations (getState() above
  // returned snapshots that are now stale).
  forcePageResync()

  // Clean up
  activeSession = null
  setInsertionIndicator(null)
  setContainerHighlight(null)
  canvas.requestRenderAll()

  return true
}

/**
 * Multi-selection version of checkDragIntoTarget.
 * Uses a given center point and list of node IDs.
 */
export function checkDragIntoTargetMulti(
  centerX: number,
  centerY: number,
  nodeIds: string[],
  canvas: fabric.Canvas,
): void {
  if (nodeIds.length === 0) return

  const store = useDocumentStore.getState()
  const nodeIdSet = new Set(nodeIds)

  let bestContainerId: string | null = null
  let bestInfo: LayoutContainerInfo | null = null
  let bestFrameBounds: { x: number; y: number; w: number; h: number } | null = null
  let bestArea = Infinity
  let bestIsLayout = false

  // 1. Check layout containers (preferred — support insertion indicators)
  for (const [containerId, info] of layoutContainerBounds) {
    if (nodeIdSet.has(containerId)) continue
    if (nodeIds.some((nid) => store.isDescendantOf(containerId, nid))) continue

    if (
      centerX >= info.x &&
      centerX <= info.x + info.w &&
      centerY >= info.y &&
      centerY <= info.y + info.h
    ) {
      const area = info.w * info.h
      if (area < bestArea) {
        bestArea = area
        bestContainerId = containerId
        bestInfo = info
        bestIsLayout = true
      }
    }
  }

  // 2. Check non-layout root frames
  for (const [frameId, bounds] of rootFrameBounds) {
    if (layoutContainerBounds.has(frameId)) continue
    if (nodeIdSet.has(frameId)) continue
    if (nodeIds.some((nid) => store.isDescendantOf(frameId, nid))) continue

    if (
      centerX >= bounds.x &&
      centerX <= bounds.x + bounds.w &&
      centerY >= bounds.y &&
      centerY <= bounds.y + bounds.h
    ) {
      const area = bounds.w * bounds.h
      if (area < bestArea) {
        bestArea = area
        bestContainerId = frameId
        bestInfo = null
        bestFrameBounds = bounds
        bestIsLayout = false
      }
    }
  }

  if (bestContainerId && bestIsLayout && bestInfo) {
    // Layout container: show insertion indicator + container highlight
    const fabObjectMap = buildFabObjectMap(canvas)
    const container = store.getNodeById(bestContainerId)
    const childIds =
      container && 'children' in container && container.children
        ? container.children.map((c) => c.id).filter((id) => !nodeIdSet.has(id))
        : []

    const isVertical = bestInfo.layout === 'vertical'
    const mainCenter = isVertical ? centerY : centerX
    const insertIndex = calcInsertionIndex(
      mainCenter,
      bestInfo,
      childIds,
      fabObjectMap,
    )

    activeSession = {
      nodeId: nodeIds[0],
      nodeIds,
      targetContainerId: bestContainerId,
      insertionIndex: insertIndex,
      isLayoutTarget: true,
    }

    computeIndicator(bestInfo, childIds, insertIndex, fabObjectMap)
    setContainerHighlight({
      x: bestInfo.x,
      y: bestInfo.y,
      w: bestInfo.w,
      h: bestInfo.h,
    })

    canvas.requestRenderAll()
  } else if (bestContainerId && !bestIsLayout) {
    // Non-layout frame: show container highlight only, append at end
    const bounds = bestFrameBounds ?? rootFrameBounds.get(bestContainerId)!
    const container = store.getNodeById(bestContainerId)
    const childIds =
      container && 'children' in container && container.children
        ? container.children.map((c) => c.id).filter((id) => !nodeIdSet.has(id))
        : []

    activeSession = {
      nodeId: nodeIds[0],
      nodeIds,
      targetContainerId: bestContainerId,
      insertionIndex: childIds.length,
      isLayoutTarget: false,
    }

    setInsertionIndicator(null)
    setContainerHighlight({
      x: bounds.x,
      y: bounds.y,
      w: bounds.w,
      h: bounds.h,
    })

    canvas.requestRenderAll()
  } else {
    if (activeSession) {
      activeSession = null
      setInsertionIndicator(null)
      setContainerHighlight(null)
      canvas.requestRenderAll()
    }
  }
}

/**
 * Commit drag-into for multi-selection: reparent or reorder nodes.
 * Handles both cross-container reparent and same-container reorder.
 * Returns true if a reparent/reorder was performed.
 */
export function commitDragIntoMulti(
  canvas: fabric.Canvas,
): boolean {
  if (!activeSession || !activeSession.nodeIds) return false

  const { nodeIds, targetContainerId, insertionIndex, isLayoutTarget } = activeSession
  const store = useDocumentStore.getState()

  // Check if this is a same-container reorder
  const firstParent = store.getParentOf(nodeIds[0])
  const isSameContainer = firstParent?.id === targetContainerId

  setFabricSyncLock(true)

  if (isSameContainer) {
    // Same container: reorder by manipulating children array directly.
    // This avoids shifting-index issues that arise when calling moveNode
    // in a loop (each removal shifts subsequent indices).
    const container = store.getNodeById(targetContainerId)
    if (container && 'children' in container && container.children) {
      const nodeIdSet = new Set(nodeIds)
      const dragged: typeof container.children = []
      const remaining: typeof container.children = []

      for (const child of container.children) {
        if (nodeIdSet.has(child.id)) {
          dragged.push(child)
        } else {
          remaining.push(child)
        }
      }

      // insertionIndex was computed for the filtered (remaining) list
      const newChildren = [
        ...remaining.slice(0, insertionIndex),
        ...dragged,
        ...remaining.slice(insertionIndex),
      ]

      store.updateNode(targetContainerId, {
        children: newChildren,
      } as Partial<import('@/types/pen').PenNode>)
    }
  } else {
    // Cross container: reparent
    const fabObjectMap = buildFabObjectMap(canvas)
    const targetBounds = rootFrameBounds.get(targetContainerId)

    for (let i = 0; i < nodeIds.length; i++) {
      if (isLayoutTarget) {
        // Clear manual position so layout engine takes over
        useDocumentStore.getState().updateNode(nodeIds[i], {
          x: undefined,
          y: undefined,
        } as Partial<import('@/types/pen').PenNode>)
      } else if (targetBounds) {
        // Non-layout frame: convert absolute position to relative
        const fabObj = fabObjectMap.get(nodeIds[i])
        if (fabObj) {
          useDocumentStore.getState().updateNode(nodeIds[i], {
            x: (fabObj.left ?? 0) - targetBounds.x,
            y: (fabObj.top ?? 0) - targetBounds.y,
          })
        }
      }

      // Reparent into the target container at successive indices
      useDocumentStore.getState().moveNode(nodeIds[i], targetContainerId, insertionIndex + i)
    }
  }

  setFabricSyncLock(false)

  // Force re-sync
  forcePageResync()

  // Clean up
  activeSession = null
  setInsertionIndicator(null)
  setContainerHighlight(null)
  canvas.requestRenderAll()

  return true
}

/** Cancel the drag-into session (safety cleanup). */
export function cancelDragInto(): void {
  if (!activeSession) return
  activeSession = null
  setInsertionIndicator(null)
  setContainerHighlight(null)
}

/** Check if a drag-into session is currently active. */
export function isDragIntoActive(): boolean {
  return activeSession !== null
}
