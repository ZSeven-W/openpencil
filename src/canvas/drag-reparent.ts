import { useDocumentStore } from '@/stores/document-store'
import { setFabricSyncLock } from './canvas-sync-lock'
import { nodeRenderInfo, rootFrameBounds, layoutContainerBounds } from './use-canvas-sync'
import type { FabricObjectWithPenId } from './canvas-object-factory'

interface Bounds {
  x: number
  y: number
  w: number
  h: number
}

function isCompletelyOutside(obj: Bounds, parent: Bounds): boolean {
  return (
    obj.x + obj.w <= parent.x ||
    obj.x >= parent.x + parent.w ||
    obj.y + obj.h <= parent.y ||
    obj.y >= parent.y + parent.h
  )
}

function overlapArea(a: Bounds, b: Bounds): number {
  const overlapX = Math.max(
    0,
    Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x),
  )
  const overlapY = Math.max(
    0,
    Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y),
  )
  return overlapX * overlapY
}

function toNumber(value: unknown): number {
  if (typeof value === 'number') return value
  if (typeof value === 'string') {
    const n = parseFloat(value)
    return Number.isFinite(n) ? n : 0
  }
  return 0
}

function getParentBounds(parentId: string, parentNode: unknown): Bounds | null {
  const root = rootFrameBounds.get(parentId)
  if (root) return root

  const layout = layoutContainerBounds.get(parentId)
  if (layout) {
    return { x: layout.x, y: layout.y, w: layout.w, h: layout.h }
  }

  if (
    !parentNode ||
    typeof parentNode !== 'object' ||
    !('x' in parentNode) ||
    !('y' in parentNode)
  ) {
    return null
  }

  const parentInfo = nodeRenderInfo.get(parentId)
  const x = toNumber((parentNode as { x?: unknown }).x) + (parentInfo?.parentOffsetX ?? 0)
  const y = toNumber((parentNode as { y?: unknown }).y) + (parentInfo?.parentOffsetY ?? 0)
  const w = toNumber((parentNode as { width?: unknown }).width)
  const h = toNumber((parentNode as { height?: unknown }).height)

  if (w <= 0 || h <= 0) return null
  return { x, y, w, h }
}

/**
 * Check if a node (identified by absolute bounds) was dragged outside its
 * parent container. If so, reparent it to the best overlapping root frame
 * or to root level.
 *
 * Unlike `checkDragReparent` (which reads Fabric coords), this version
 * accepts pre-computed absolute bounds — needed for objects inside an
 * ActiveSelection whose Fabric left/top are group-relative.
 *
 * The caller is responsible for holding the fabric sync lock.
 * Returns true if reparenting occurred.
 */
export function checkDragReparentByBounds(
  nodeId: string,
  objBounds: Bounds,
): boolean {
  const store = useDocumentStore.getState()
  const parent = store.getParentOf(nodeId)
  if (!parent) return false // Already root-level

  const parentBounds = getParentBounds(parent.id, parent)
  if (!parentBounds) return false

  if (!isCompletelyOutside(objBounds, parentBounds)) return false

  // Find the root frame with the most overlap
  let bestFrameId: string | null = null
  let bestOverlap = 0

  for (const [frameId, frameBounds] of rootFrameBounds) {
    // Don't reparent into the frame we just left
    if (frameId === parent.id) continue
    const area = overlapArea(objBounds, frameBounds)
    if (area > bestOverlap) {
      bestOverlap = area
      bestFrameId = frameId
    }
  }

  if (bestFrameId) {
    // Reparent into the overlapping frame — convert absolute to relative position
    const targetBounds = rootFrameBounds.get(bestFrameId)!
    store.updateNode(nodeId, {
      x: objBounds.x - targetBounds.x,
      y: objBounds.y - targetBounds.y,
    })
    const targetChildren = store.getNodeById(bestFrameId)
    const childCount =
      targetChildren && 'children' in targetChildren && targetChildren.children
        ? targetChildren.children.length
        : 0
    store.moveNode(nodeId, bestFrameId, childCount)
  } else {
    // No overlapping frame — make it a root-level node
    store.updateNode(nodeId, {
      x: objBounds.x,
      y: objBounds.y,
    })
    const rootCount = store.document.children.length
    store.moveNode(nodeId, null, rootCount)
  }

  return true
}

/**
 * Check if a root-level node (identified by absolute bounds) should be
 * reparented INTO a root frame based on center-point containment.
 *
 * This handles the case where objects are dragged from root level into
 * a frame — the reverse of checkDragReparentByBounds (which handles
 * objects being dragged OUT of a frame).
 *
 * The caller is responsible for holding the fabric sync lock.
 * Returns true if reparenting occurred.
 */
export function checkReparentIntoFrame(
  nodeId: string,
  objBounds: Bounds,
  skipFrameIds?: Set<string>,
): boolean {
  const store = useDocumentStore.getState()
  const parent = store.getParentOf(nodeId)
  if (parent) return false // Not root-level — use checkDragReparentByBounds instead

  const centerX = objBounds.x + objBounds.w / 2
  const centerY = objBounds.y + objBounds.h / 2

  // Find the smallest root frame containing the object's center
  let bestFrameId: string | null = null
  let bestArea = Infinity

  for (const [frameId, frameBounds] of rootFrameBounds) {
    if (frameId === nodeId) continue
    if (skipFrameIds?.has(frameId)) continue
    if (store.isDescendantOf(frameId, nodeId)) continue

    if (
      centerX >= frameBounds.x &&
      centerX <= frameBounds.x + frameBounds.w &&
      centerY >= frameBounds.y &&
      centerY <= frameBounds.y + frameBounds.h
    ) {
      const area = frameBounds.w * frameBounds.h
      if (area < bestArea) {
        bestArea = area
        bestFrameId = frameId
      }
    }
  }

  // Also check layout containers (nested layout frames that aren't root frames)
  for (const [containerId, info] of layoutContainerBounds) {
    if (containerId === nodeId) continue
    if (skipFrameIds?.has(containerId)) continue
    if (rootFrameBounds.has(containerId)) continue // already checked above
    if (store.isDescendantOf(containerId, nodeId)) continue

    if (
      centerX >= info.x &&
      centerX <= info.x + info.w &&
      centerY >= info.y &&
      centerY <= info.y + info.h
    ) {
      const area = info.w * info.h
      if (area < bestArea) {
        bestArea = area
        bestFrameId = containerId
      }
    }
  }

  if (!bestFrameId) return false

  // Determine if target is a layout container (clear x/y) or plain frame (relative position)
  const layoutInfo = layoutContainerBounds.get(bestFrameId)
  if (layoutInfo) {
    // Layout container: clear position, layout engine will place it
    store.updateNode(nodeId, {
      x: undefined,
      y: undefined,
    } as any)
  } else {
    // Non-layout frame: convert absolute to relative position
    const targetBounds = rootFrameBounds.get(bestFrameId)
    if (targetBounds) {
      store.updateNode(nodeId, {
        x: objBounds.x - targetBounds.x,
        y: objBounds.y - targetBounds.y,
      })
    }
  }

  const targetChildren = store.getNodeById(bestFrameId)
  const childCount =
    targetChildren && 'children' in targetChildren && targetChildren.children
      ? targetChildren.children.length
      : 0
  store.moveNode(nodeId, bestFrameId, childCount)

  return true
}

/**
 * Check if a Fabric object was dragged outside its current parent container.
 * If so, reparent it to the overlapping root frame (if any), or to root level.
 *
 * Works for layout and non-layout containers.
 * Returns true if reparenting occurred.
 */
export function checkDragReparent(obj: FabricObjectWithPenId): boolean {
  const nodeId = obj.penNodeId
  if (!nodeId) return false

  const store = useDocumentStore.getState()
  const parent = store.getParentOf(nodeId)
  if (!parent) return false // Already root-level

  const parentBounds = getParentBounds(parent.id, parent)
  if (!parentBounds) return false

  // Compute object's absolute bounds from Fabric
  const objBounds: Bounds = {
    x: obj.left ?? 0,
    y: obj.top ?? 0,
    w: (obj.width ?? 0) * (obj.scaleX ?? 1),
    h: (obj.height ?? 0) * (obj.scaleY ?? 1),
  }

  // Only trigger if completely outside parent
  if (!isCompletelyOutside(objBounds, parentBounds)) return false

  // Find the root frame with the most overlap
  let bestFrameId: string | null = null
  let bestOverlap = 0

  for (const [frameId, frameBounds] of rootFrameBounds) {
    const area = overlapArea(objBounds, frameBounds)
    if (area > bestOverlap) {
      bestOverlap = area
      bestFrameId = frameId
    }
  }

  setFabricSyncLock(true)
  try {
    if (bestFrameId) {
      // Reparent into the overlapping frame — convert absolute to relative position
      const targetBounds = rootFrameBounds.get(bestFrameId)!
      store.updateNode(nodeId, {
        x: objBounds.x - targetBounds.x,
        y: objBounds.y - targetBounds.y,
      })
      const targetChildren = store.getNodeById(bestFrameId)
      const childCount =
        targetChildren && 'children' in targetChildren && targetChildren.children
          ? targetChildren.children.length
          : 0
      store.moveNode(nodeId, bestFrameId, childCount)
    } else {
      // No overlapping frame — make it a root-level node
      store.updateNode(nodeId, {
        x: objBounds.x,
        y: objBounds.y,
      })
      const rootCount = store.document.children.length
      store.moveNode(nodeId, null, rootCount)
    }
  } finally {
    setFabricSyncLock(false)
  }

  return true
}
