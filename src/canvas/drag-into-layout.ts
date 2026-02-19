import type * as fabric from 'fabric'
import { useDocumentStore } from '@/stores/document-store'
import type { FabricObjectWithPenId } from './canvas-object-factory'
import { setFabricSyncLock } from './canvas-sync-lock'
import { layoutContainerBounds } from './use-canvas-sync'
import type { LayoutContainerInfo } from './use-canvas-sync'
import { setInsertionIndicator, setContainerHighlight } from './insertion-indicator'

// ---------------------------------------------------------------------------
// Session state
// ---------------------------------------------------------------------------

interface DragIntoSession {
  nodeId: string
  targetContainerId: string
  insertionIndex: number
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

  // Find the best (innermost) layout container containing the center.
  // Innermost = smallest area among matching containers.
  let bestContainerId: string | null = null
  let bestInfo: LayoutContainerInfo | null = null
  let bestArea = Infinity

  for (const [containerId, info] of layoutContainerBounds) {
    // Skip if the dragged node is already a direct child of this container
    const parent = store.getParentOf(nodeId)
    if (parent?.id === containerId) continue

    // Skip if the container is a descendant of the dragged node (prevent cycles)
    if (store.isDescendantOf(containerId, nodeId)) continue

    // Hit test: is the center inside the container bounds?
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
      }
    }
  }

  if (bestContainerId && bestInfo) {
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
    }

    computeIndicator(bestInfo, childIds, insertIndex, fabObjectMap)
    setContainerHighlight({
      x: bestInfo.x,
      y: bestInfo.y,
      w: bestInfo.w,
      h: bestInfo.h,
    })

    canvas.requestRenderAll()
  } else {
    // Not over any layout container — clear
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
  _obj: FabricObjectWithPenId,
  canvas: fabric.Canvas,
): boolean {
  if (!activeSession) return false

  const { nodeId, targetContainerId, insertionIndex } = activeSession

  setFabricSyncLock(true)

  // Clear manual position so layout engine takes over
  useDocumentStore.getState().updateNode(nodeId, {
    x: undefined,
    y: undefined,
  } as Partial<import('@/types/pen').PenNode>)

  // Reparent into the target container
  useDocumentStore.getState().moveNode(nodeId, targetContainerId, insertionIndex)

  setFabricSyncLock(false)

  // Force re-sync: must re-read state after mutations (getState() above
  // returned snapshots that are now stale).
  const doc = useDocumentStore.getState().document
  useDocumentStore.setState({
    document: { ...doc, children: [...doc.children] },
  })

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
